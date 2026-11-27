//! 설정 디렉터리 로딩.

use std::path::{Path, PathBuf};

use fruit_star_core::Symbol;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::application::ApplicationConfig;
use crate::audio::AudioConfig;
use crate::controls::ControlsConfig;
use crate::error::ConfigError;
use crate::graphics::GraphicsConfig;
use crate::machine::MachineConfig;

/// 설정 파일에 문제가 있을 때의 처리 방식.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    /// 오류를 그대로 돌려준다.
    Strict,
    /// 문제가 있는 파일은 기본값으로 대체하고 경고를 남긴다.
    UseDefaults,
}

/// 전체 설정.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConfigSet {
    pub application: ApplicationConfig,
    pub graphics: GraphicsConfig,
    pub audio: AudioConfig,
    pub controls: ControlsConfig,
    pub machine: MachineConfig,
}

/// 로딩 결과와 그 과정에서 생긴 경고.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadReport {
    pub config: ConfigSet,
    pub warnings: Vec<String>,
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(ConfigError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    toml::from_str(&text)
        .map(Some)
        .map_err(|error| ConfigError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

/// 파일 하나를 읽고 검증한다. 파일이 없으면 기본값을 쓰고 경고를 남긴다.
fn load_one<T: DeserializeOwned + Default>(
    path: &Path,
    recovery: Recovery,
    warnings: &mut Vec<String>,
    validate: impl Fn(&T) -> Result<(), ConfigError>,
) -> Result<T, ConfigError> {
    let loaded = read::<T>(path).and_then(|value| match value {
        Some(value) => validate(&value).map(|()| Some(value)),
        None => Ok(None),
    });
    match loaded {
        Ok(Some(value)) => Ok(value),
        Ok(None) => {
            warnings.push(format!(
                "설정 파일 {}이(가) 없어 기본값을 사용합니다.",
                path.display()
            ));
            Ok(T::default())
        }
        Err(error) => match recovery {
            Recovery::Strict => Err(error),
            Recovery::UseDefaults => {
                warnings.push(format!("{error} → 기본값을 사용합니다."));
                Ok(T::default())
            }
        },
    }
}

impl ConfigSet {
    /// `dir`(보통 `config/`)에서 모든 설정을 읽는다.
    pub fn load(dir: &Path, recovery: Recovery) -> Result<LoadReport, ConfigError> {
        Self::load_for(dir, recovery, None)
    }

    /// [`ConfigSet::load`]와 같되, `platform`이 주어지면 `dir/platform/<platform>/`에 같은
    /// 이름의 파일이 있을 때 그것을 대신 읽는다 (예: 휴대폰용 그래픽 설정).
    pub fn load_for(
        dir: &Path,
        recovery: Recovery,
        platform: Option<&str>,
    ) -> Result<LoadReport, ConfigError> {
        let pick = |name: &str| {
            platform
                .map(|p| dir.join("platform").join(p).join(name))
                .filter(|path| path.is_file())
                .unwrap_or_else(|| dir.join(name))
        };
        let mut warnings = Vec::new();
        let application: ApplicationConfig = load_one(
            &pick("application.toml"),
            recovery,
            &mut warnings,
            ApplicationConfig::validate,
        )?;
        let graphics = load_one(
            &pick("graphics.toml"),
            recovery,
            &mut warnings,
            GraphicsConfig::validate,
        )?;
        let audio = load_one(
            &pick("audio.toml"),
            recovery,
            &mut warnings,
            AudioConfig::validate,
        )?;
        let controls = load_one(
            &pick("controls.toml"),
            recovery,
            &mut warnings,
            ControlsConfig::validate,
        )?;
        let machine_path = dir
            .join("machines")
            .join(format!("{}.toml", application.game.machine));
        let machine = load_one(
            &machine_path,
            recovery,
            &mut warnings,
            MachineConfig::validate,
        )?;
        Ok(LoadReport {
            config: ConfigSet {
                application,
                graphics,
                audio,
                controls,
                machine,
            },
            warnings,
        })
    }

    /// 전체 설정을 검증한다.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.application.validate()?;
        self.graphics.validate()?;
        self.audio.validate()?;
        self.controls.validate()?;
        self.machine.validate()
    }

    /// 기본 설정 파일을 `dir`에 기록한다 (기본값 복구용). 기존 파일은 덮어쓴다.
    pub fn write_defaults(dir: &Path) -> Result<Vec<PathBuf>, ConfigError> {
        let defaults = ConfigSet::default();
        let machine_file = format!("machines/{}.toml", defaults.machine.id);
        let mut written = Vec::new();
        write_toml(dir, "application.toml", &defaults.application, &mut written)?;
        write_toml(dir, "graphics.toml", &defaults.graphics, &mut written)?;
        write_toml(dir, "audio.toml", &defaults.audio, &mut written)?;
        write_toml(dir, "controls.toml", &defaults.controls, &mut written)?;
        write_toml(dir, &machine_file, &defaults.machine, &mut written)?;
        Ok(written)
    }
}

fn write_toml<T: Serialize>(
    dir: &Path,
    name: &str,
    value: &T,
    written: &mut Vec<PathBuf>,
) -> Result<(), ConfigError> {
    let path = dir.join(name);
    let text = toml::to_string_pretty(value).map_err(|e| ConfigError::Serialize(e.to_string()))?;
    let io_error = |source| ConfigError::Io {
        path: path.clone(),
        source,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_error)?;
    }
    std::fs::write(&path, text).map_err(io_error)?;
    written.push(path);
    Ok(())
}

/// 실행에 반드시 필요한 리소스 파일 (assets 디렉터리 기준 상대 경로).
pub fn required_assets() -> Vec<String> {
    let mut assets: Vec<String> = Symbol::ALL
        .iter()
        .map(|symbol| format!("symbols/{}.png", symbol.key()))
        .collect();
    for texture in [
        "menu_marquee",
        "top_panel",
        "belly_panel",
        "reel_band",
        "star_lamp",
        "medal_face",
        "labels/credit",
        "labels/payout",
        "labels/count",
        "labels/star_chance",
        "labels/max_bet",
        "labels/start",
        "labels/stop",
        "labels/insert",
        "labels/one_bet",
        "labels/replay",
        "labels/games",
        "labels/big",
        "labels/reg",
        "labels/total",
        "paytable",
    ] {
        assets.push(format!("textures/{texture}.png"));
    }
    for sound in [
        "power_on",
        "ambient",
        "lever_pull",
        "lever_return",
        "reel_start",
        "reel_loop",
        "reel_stop",
        "button_press",
        "button_release",
        "medal_insert",
        "medal_drop",
        "medal_collision",
        "bonus_start",
        "bonus_end",
        "menu_select",
        "win_small",
        "win_big",
        "replay",
        "bet",
        "bonus_loop",
        "chance_lamp",
    ] {
        assets.push(format!("audio/{sound}.wav"));
    }
    assets
}

/// `assets_dir`에 없는 필수 리소스 목록.
pub fn missing_assets(assets_dir: &Path) -> Vec<String> {
    required_assets()
        .into_iter()
        .filter(|relative| !assets_dir.join(relative).is_file())
        .collect()
}
