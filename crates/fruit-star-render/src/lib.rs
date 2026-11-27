//! FRUIT STAR 3D 렌더링 계층.
//!
//! 캐비닛과 릴을 만들고, 게임기 계층이 계산한 논리 상태(릴 위치, 눌림, 점등)를
//! 메시와 재질에 반영한다. 게임 규칙은 전혀 알지 못한다.

pub mod assets;
pub mod cabinet;
pub mod camera;
pub mod effects;
pub mod image_ops;
pub mod lighting;
pub mod materials;
pub mod reel_mesh;
pub mod warmup;

use bevy::picking::mesh_picking::{MeshPickingPlugin, MeshPickingSettings};
use bevy::prelude::*;
use fruit_star_machine::{AppState, DebugLevel, Machine, MachineSet, Settings};

pub use assets::{ArtAssets, Label};
pub use cabinet::{
    CabinetRoot, CabinetSpec, PayLineBar, REQUIRED_PARTS, ReelDrum, ShellPart, spawn_cabinet,
};
pub use camera::{CameraRig, OrbitPose};
pub use effects::MedalMesh;
pub use materials::MaterialLibrary;

// 릴리스 빌드에서는 디버그 단계를 쓰지 않는다.
#[cfg(not(debug_assertions))]
const _: fn() = || {
    let _ = std::mem::size_of::<DebugLevel>();
};

/// 렌더링 플러그인.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // 렌더 월드가 파이프라인 현황을 알려 주면 로딩 화면이 예열 완료를 판단한다.
        let progress = warmup::PipelineProgress::new();
        app.insert_resource(progress.clone());
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.insert_resource(progress).add_systems(
                bevy::render::Render,
                warmup::report_pipelines.after(bevy::render::RenderSystems::Render),
            );
        }
        app.add_plugins(MeshPickingPlugin)
            .insert_resource(MeshPickingSettings {
                // 표시가 붙은 부품과 카메라만 대상으로 삼아 광선 검사를 줄인다.
                require_markers: true,
                ..default()
            })
            .add_systems(
                Startup,
                (
                    lighting::setup_environment,
                    (camera::spawn_camera, lighting::spawn_lights),
                )
                    .chain(),
            )
            .add_systems(OnEnter(AppState::Loading), assets::begin_loading)
            .add_systems(
                Update,
                (
                    assets::finish_loading,
                    spawn_scene.run_if(
                        resource_exists::<assets::ArtAssets>
                            .and_then(not(resource_exists::<materials::MaterialLibrary>)),
                    ),
                    warmup::run_warmup,
                )
                    .chain()
                    .run_if(in_state(AppState::Loading)),
            )
            .add_systems(
                Update,
                lighting::show_neighbors.run_if(state_changed::<AppState>),
            )
            .add_systems(
                Update,
                (
                    effects::sync_reels,
                    effects::apply_press_motion,
                    effects::apply_segments,
                    effects::apply_lamps,
                    effects::apply_view_mode,
                    effects::apply_payline_visibility
                        .run_if(not(resource_exists::<warmup::Warmup>)),
                    effects::spawn_inserted_medals,
                    effects::animate_inserted_medals,
                    effects::dress_medals,
                )
                    .in_set(MachineSet::Present)
                    .run_if(
                        resource_exists::<MaterialLibrary>.and_then(resource_exists::<MedalMesh>),
                    ),
            )
            .add_systems(
                Update,
                (camera::follow_view_mode, camera::control_camera)
                    .chain()
                    .in_set(MachineSet::Present),
            );

        #[cfg(debug_assertions)]
        app.add_plugins(bevy::pbr::wireframe::WireframePlugin::default())
            .add_systems(Update, apply_wireframe);
    }
}

/// 디버그 단계 2에서 와이어프레임을 켠다 (개발 빌드 전용).
#[cfg(debug_assertions)]
fn apply_wireframe(
    level: Res<DebugLevel>,
    mut config: ResMut<bevy::pbr::wireframe::WireframeConfig>,
) {
    let wanted = level.0 >= 2;
    if config.global != wanted {
        config.global = wanted;
    }
}

/// 로딩이 끝나면 재질을 만들고 게임기와 주변 공간을 배치한다.
fn spawn_scene(
    mut commands: Commands,
    art: Option<Res<ArtAssets>>,
    machine: Res<Machine>,
    settings: Res<Settings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 로딩 실패로 빠져나온 경우에는 만들 것이 없다.
    let Some(art) = art else {
        return;
    };
    let library = MaterialLibrary::new(&mut materials, &art);
    let spec = CabinetSpec {
        library: &library,
        art: &art,
        symbol_count: machine.profile().symbol_count,
        reel_segments: settings.config.graphics.render.quality.reel_segments(),
        animation: &settings.config.machine.animation,
    };
    spawn_cabinet(&mut commands, &mut meshes, &mut materials, &spec);
    lighting::spawn_room(&mut commands, &mut meshes, &mut materials, &library);
    commands.insert_resource(MedalMesh(meshes.add(effects::medal_mesh())));
    commands.insert_resource(library);
}
