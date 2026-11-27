//! FRUIT STAR 3D 실행 파일.
//!
//! 여기서는 설정을 읽고 플러그인을 조립하기만 한다. 기능은 각 크레이트에 있다.

mod automation;
mod cli;
mod paths;
mod persistence;
mod window;

use std::process::ExitCode;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use fruit_star_config::{ConfigSet, Recovery};
use fruit_star_game::PlayMode;
use fruit_star_machine::{Machine, MachinePlugin, Settings};

use crate::automation::{Automation, AutomationPlugin};
use crate::persistence::{PersistencePlugin, RandomSeed};
use crate::window::WindowControlPlugin;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match cli::parse(args.iter().map(String::as_str)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    if options.help {
        println!("{}", cli::USAGE);
        return ExitCode::SUCCESS;
    }

    let root = paths::resource_root();
    // 문제가 있는 설정 파일은 기본값으로 대체하고 경고를 남긴다.
    // 휴대폰은 `config/platform/ios/`의 파일(가벼운 그래픽 설정 등)을 우선 읽는다.
    let platform = cfg!(target_os = "ios").then_some("ios");
    let report = match ConfigSet::load_for(&root.join("config"), Recovery::UseDefaults, platform) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("설정을 읽을 수 없습니다: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut config = report.config;
    let mut warnings = report.warnings;

    let (persistence, store_warnings) = match persistence::open_store(options.database.as_deref()) {
        Ok(opened) => opened,
        Err(error) => {
            eprintln!("저장소를 준비할 수 없습니다: {error}");
            return ExitCode::FAILURE;
        }
    };
    warnings.extend(store_warnings);
    persistence::apply_saved_settings(&persistence, &mut config);

    let profile = match config.machine.to_profile() {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("기종 설정이 올바르지 않습니다: {error}");
            return ExitCode::FAILURE;
        }
    };
    let seed = RandomSeed(options.seed);
    let mode = PlayMode::from_key(&config.application.game.mode).unwrap_or_default();
    let controller = match persistence::build_controller(
        &persistence,
        &config,
        profile,
        config.machine.reel_timing(),
        mode,
        seed,
    ) {
        Ok(controller) => controller,
        Err(error) => {
            eprintln!("게임기를 준비할 수 없습니다: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(window::primary_window(&config)),
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.join("assets").to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .insert_resource(Machine(controller))
    .insert_resource(Settings {
        config,
        root: root.clone(),
    })
    .insert_resource(persistence)
    .insert_resource(seed)
    .insert_resource(Automation::from_options(&options))
    .insert_resource(StartupWarnings(warnings))
    .add_plugins((
        MachinePlugin,
        fruit_star_render::RenderPlugin,
        fruit_star_input::InputPlugin,
        fruit_star_audio::AudioPlugin,
        fruit_star_physics::PhysicsPlugin,
        fruit_star_ui::UiPlugin,
        PersistencePlugin,
        WindowControlPlugin,
        AutomationPlugin,
    ))
    .add_systems(Startup, log_startup);
    if options.offscreen {
        let (width, height) = options.offscreen_size.unwrap_or((1920, 1080));
        app.add_systems(
            PreStartup,
            move |commands: Commands, images: ResMut<Assets<Image>>| {
                create_offscreen_target(commands, images, width, height);
            },
        );
    }
    if options.offscreen || options.auto_play {
        // 사람이 보지 않는 자동 실행은 처음부터 창 포커스가 없다.
        app.insert_resource(fruit_star_input::PauseOnFocusLoss(false));
    }
    match app.run() {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(code) => ExitCode::from(code.get()),
    }
}

/// 시작 과정에서 생긴 경고 (설정 복구, 저장소 복구 등).
#[derive(Resource, Debug, Clone, Default)]
struct StartupWarnings(Vec<String>);

fn log_startup(warnings: Res<StartupWarnings>, settings: Res<Settings>) {
    info!("리소스 루트: {}", settings.root.display());
    for warning in &warnings.0 {
        warn!("{warning}");
    }
}

/// 화면 밖 렌더링 대상을 만든다 (`--offscreen`).
fn create_offscreen_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    width: u32,
    height: u32,
) {
    use bevy::render::render_resource::TextureFormat;
    let image = Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None);
    commands.insert_resource(fruit_star_machine::OffscreenTarget(images.add(image)));
}
