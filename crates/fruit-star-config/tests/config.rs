#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use fruit_star_config::{
    ApplicationConfig, AudioConfig, ConfigError, ConfigSet, ControlsConfig, GraphicsConfig,
    KNOWN_KEYS, Language, MachineConfig, Quality, Recovery, missing_assets, required_assets,
};
use fruit_star_core::ReelTiming;
use fruit_star_game::MachineProfile;

fn shipped_config_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config")
}

/// 테스트마다 고유한 임시 디렉터리.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fruit-star-config-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn shipped_files_load_strictly_and_match_builtin_defaults() {
    let report = ConfigSet::load(&shipped_config_dir(), Recovery::Strict).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(report.config, ConfigSet::default());
    report.config.validate().unwrap();
}

#[test]
fn shipped_machine_matches_domain_profile() {
    let report = ConfigSet::load(&shipped_config_dir(), Recovery::Strict).unwrap();
    let profile = report.config.machine.to_profile().unwrap();
    let builtin = MachineProfile::fruit_classic();
    assert_eq!(profile.strips, builtin.strips);
    assert_eq!(profile.paytable, builtin.paytable);
    assert_eq!(profile.bet, builtin.bet);
    assert_eq!(profile.max_slip, builtin.max_slip);
    assert_eq!(profile.bonus.big_payout_cap, builtin.bonus.big_payout_cap);
    for (flag, weight) in builtin.lottery.entries() {
        assert_eq!(profile.lottery.weight(*flag), *weight, "{flag:?}");
    }
    assert_eq!(
        profile.lottery.entries().len(),
        builtin.lottery.entries().len()
    );
    assert_eq!(report.config.machine.reel_timing(), ReelTiming::default());
}

#[test]
fn missing_files_fall_back_to_defaults_with_warnings() {
    let dir = temp_dir("missing");
    let report = ConfigSet::load(&dir, Recovery::Strict).unwrap();
    assert_eq!(report.config, ConfigSet::default());
    assert_eq!(report.warnings.len(), 5);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn platform_files_override_the_shared_ones() {
    let dir = temp_dir("platform");
    ConfigSet::write_defaults(&dir).unwrap();
    std::fs::create_dir_all(dir.join("platform/ios")).unwrap();
    std::fs::write(
        dir.join("platform/ios/graphics.toml"),
        "[render]\nquality = \"medium\"\ntarget_fps = 60\n",
    )
    .unwrap();
    let shared = ConfigSet::load(&dir, Recovery::Strict).unwrap();
    assert_eq!(shared.config.graphics.render.target_fps, 120);
    let ios = ConfigSet::load_for(&dir, Recovery::Strict, Some("ios")).unwrap();
    assert_eq!(ios.config.graphics.render.target_fps, 60);
    assert_eq!(ios.config.graphics.render.quality, Quality::Medium);
    // 덮어쓰지 않은 파일은 공용 파일을 그대로 쓴다
    assert_eq!(ios.config.audio, shared.config.audio);
    // 없는 플랫폼은 공용 파일과 같다
    let other = ConfigSet::load_for(&dir, Recovery::Strict, Some("android")).unwrap();
    assert_eq!(other.config, shared.config);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn shipped_ios_graphics_file_is_valid() {
    let report = ConfigSet::load_for(&shipped_config_dir(), Recovery::Strict, Some("ios")).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let render = &report.config.graphics.render;
    assert!(render.target_fps <= 60);
    assert!(
        render.hot_target_fps < render.target_fps,
        "뜨거울 때의 목표가 평소보다 낮아야 열을 줄인다"
    );
    assert!(!render.ambient_occlusion && !render.motion_blur);
}

#[test]
fn hot_target_fps_out_of_range_is_rejected() {
    for bad in ["10", "300"] {
        let dir = temp_dir(&format!("hot_fps_{bad}"));
        std::fs::write(
            dir.join("graphics.toml"),
            format!("[render]\nhot_target_fps = {bad}\n"),
        )
        .unwrap();
        let message = ConfigSet::load(&dir, Recovery::Strict)
            .unwrap_err()
            .to_string();
        assert!(message.contains("hot_target_fps"), "{message}");
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn written_defaults_round_trip() {
    let dir = temp_dir("roundtrip");
    let written = ConfigSet::write_defaults(&dir).unwrap();
    assert_eq!(written.len(), 5);
    let report = ConfigSet::load(&dir, Recovery::Strict).unwrap();
    assert!(report.warnings.is_empty());
    assert_eq!(report.config, ConfigSet::default());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn invalid_file_is_an_error_in_strict_mode_and_recovers_otherwise() {
    let dir = temp_dir("invalid");
    std::fs::write(dir.join("graphics.toml"), "[render]\ntarget_fps = 5\n").unwrap();
    std::fs::write(dir.join("audio.toml"), "master = \"loud\"\n").unwrap();

    let error = ConfigSet::load(&dir, Recovery::Strict).unwrap_err();
    let message = error.to_string();
    assert!(matches!(error, ConfigError::Invalid { .. }), "{message}");
    assert!(message.contains("target_fps"), "{message}");
    assert!(
        message.contains("30"),
        "허용 범위를 알려 줘야 한다: {message}"
    );

    let report = ConfigSet::load(&dir, Recovery::UseDefaults).unwrap();
    assert_eq!(report.config.graphics, GraphicsConfig::default());
    assert_eq!(report.config.audio, AudioConfig::default());
    assert!(report.warnings.iter().any(|w| w.contains("target_fps")));
    assert!(report.warnings.iter().any(|w| w.contains("audio.toml")));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn unknown_keys_are_rejected() {
    let dir = temp_dir("unknown");
    std::fs::write(dir.join("application.toml"), "[window]\nwidht = 100\n").unwrap();
    let error = ConfigSet::load(&dir, Recovery::Strict).unwrap_err();
    assert!(matches!(error, ConfigError::Parse { .. }));
    assert!(error.to_string().contains("widht"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn partial_files_keep_defaults_for_missing_keys() {
    let dir = temp_dir("partial");
    std::fs::write(
        dir.join("application.toml"),
        "[game]\nlanguage = \"ja\"\nmode = \"practice\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("graphics.toml"), "[render]\nquality = \"ultra\"\n").unwrap();
    let report = ConfigSet::load(&dir, Recovery::Strict).unwrap();
    assert_eq!(report.config.application.game.language, Language::Ja);
    assert_eq!(report.config.application.game.mode, "practice");
    assert_eq!(report.config.application.window.width, 1920);
    assert_eq!(report.config.graphics.render.quality, Quality::Ultra);
    assert!(report.config.graphics.render.bloom);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn application_ranges_are_validated() {
    let mut config = ApplicationConfig::default();
    config.window.width = 100;
    assert!(config.validate().is_err());

    let mut config = ApplicationConfig::default();
    config.game.mode = "turbo".to_owned();
    assert!(config.validate().is_err());

    let mut config = ApplicationConfig::default();
    config.game.machine = "../etc/passwd".to_owned();
    assert!(
        config.validate().is_err(),
        "기종 ID로 경로를 벗어날 수 없어야 한다"
    );

    let mut config = ApplicationConfig::default();
    config.game.initial_credit = 20_000;
    assert!(config.validate().is_err());
}

#[test]
fn graphics_and_audio_ranges_are_validated() {
    let mut config = GraphicsConfig::default();
    config.render.render_scale = f32::NAN;
    assert!(config.validate().is_err());
    let mut config = GraphicsConfig::default();
    config.render.backend = "vulkan".to_owned();
    assert!(config.validate().is_err());
    let mut config = GraphicsConfig::default();
    config.led.brightness = 3.0;
    assert!(config.validate().is_err());

    let audio = AudioConfig {
        master: 1.5,
        ..AudioConfig::default()
    };
    assert!(audio.validate().is_err());
    let audio = AudioConfig {
        muted: true,
        ..AudioConfig::default()
    };
    assert_eq!(audio.effective_effects(), 0.0);
    assert_eq!(audio.effective_ambient(), 0.0);
}

#[test]
fn key_bindings_are_validated() {
    let defaults = ControlsConfig::default();
    defaults.validate().unwrap();
    for (_, keys) in defaults.keys.actions() {
        for key in keys {
            assert!(KNOWN_KEYS.contains(&key.as_str()));
        }
    }

    let mut config = ControlsConfig::default();
    config.keys.bet = vec!["Hyper".to_owned()];
    assert!(config.validate().is_err());

    let mut config = ControlsConfig::default();
    config.keys.bet = vec!["Space".to_owned()];
    let error = config.validate().unwrap_err().to_string();
    assert!(error.contains("Space"), "{error}");

    let mut config = ControlsConfig::default();
    config.keys.lever.clear();
    assert!(config.validate().is_err());
}

#[test]
fn machine_config_rejects_broken_rules() {
    let mut machine = MachineConfig::default();
    machine.reel.strips.left[0] = "banana".to_owned();
    let error = machine.validate().unwrap_err().to_string();
    assert!(error.contains("banana"), "{error}");

    let mut machine = MachineConfig::default();
    machine.reel.strips.center.pop();
    assert!(machine.validate().is_err());

    let mut machine = MachineConfig::default();
    machine.lottery.normal.insert("grape".to_owned(), 60_000);
    let error = machine.validate().unwrap_err().to_string();
    assert!(error.contains("lottery"), "{error}");

    let mut machine = MachineConfig::default();
    machine.lottery.normal.insert("big_red".to_owned(), 10);
    assert!(
        machine.validate().is_err(),
        "보너스 역할 이름은 추첨 항목이 아니다"
    );

    let mut machine = MachineConfig::default();
    machine.payouts.insert("grape".to_owned(), 50);
    assert!(machine.validate().is_err());

    let mut machine = MachineConfig::default();
    machine.reel.count = 5;
    assert!(machine.validate().is_err());

    let mut machine = MachineConfig::default();
    machine.bonus.lamp_lead_percent = 101;
    let error = machine.validate().unwrap_err().to_string();
    assert!(error.contains("lamp_lead_percent"), "{error}");
    machine.bonus.lamp_lead_percent = 100;
    machine.validate().unwrap();
}

#[test]
fn required_assets_are_listed_and_missing_ones_are_reported() {
    let assets = required_assets();
    assert!(assets.contains(&"symbols/red_seven.png".to_owned()));
    assert!(assets.contains(&"audio/reel_stop.wav".to_owned()));
    let dir = temp_dir("assets");
    assert_eq!(missing_assets(&dir).len(), assets.len());
    std::fs::create_dir_all(dir.join("symbols")).unwrap();
    std::fs::write(dir.join("symbols/red_seven.png"), b"x").unwrap();
    assert_eq!(missing_assets(&dir).len(), assets.len() - 1);
    std::fs::remove_dir_all(dir).unwrap();
}
