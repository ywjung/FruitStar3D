//! 논리 상태를 메시와 재질에 반영한다.

use bevy::prelude::*;
use fruit_star_game::GameEvent;
use fruit_star_machine::layout::{MEDAL_RADIUS, MEDAL_SLOT_POS, MEDAL_THICKNESS};
use fruit_star_machine::{
    AppState, Lamp, LampKind, Machine, MachineEvent, Medal, Pressable, Settings, SevenSegment,
    ViewMode,
};

use crate::cabinet::{PayLineBar, PressMotion, ReelDrum, RestPose, ShellPart};
use crate::materials::MaterialLibrary;
use crate::reel_mesh::rotation_for_position;

/// 릴 원통의 회전을 도메인의 릴 위치에 맞춘다.
pub(crate) fn sync_reels(machine: Res<Machine>, mut drums: Query<(&ReelDrum, &mut Transform)>) {
    let reels = machine.reels();
    for (drum, mut transform) in &mut drums {
        let Some(reel) = reels.get(drum.0) else {
            continue;
        };
        let angle = rotation_for_position(reel.display_position(), reel.len());
        transform.rotation = Quat::from_rotation_x(angle);
    }
}

/// 눌림 정도를 부품의 변위로 바꾼다.
pub(crate) fn apply_press_motion(
    mut parts: Query<(&Pressable, &PressMotion, &RestPose, &mut Transform), Changed<Pressable>>,
) {
    for (pressable, motion, rest, mut transform) in &mut parts {
        let mut pose = rest.0;
        match *motion {
            PressMotion::Translate(offset) => pose.translation += offset * pressable.amount,
            PressMotion::RotateX(angle) => {
                pose.rotation = rest.0.rotation * Quat::from_rotation_x(angle * pressable.amount);
            }
        }
        *transform = pose;
    }
}

/// 7세그먼트 획의 점등 상태를 재질로 반영한다.
pub(crate) fn apply_segments(
    library: Res<MaterialLibrary>,
    mut segments: Query<
        (&SevenSegment, &mut MeshMaterial3d<StandardMaterial>),
        Changed<SevenSegment>,
    >,
) {
    for (segment, mut material) in &mut segments {
        let wanted = if segment.lit {
            &library.segment_on
        } else {
            &library.segment_off
        };
        if material.0 != *wanted {
            material.0 = wanted.clone();
        }
    }
}

/// 마지막으로 재질에 반영한 램프 상태. 변화가 있을 때만 재질을 갱신하기 위한 기록이다.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct AppliedLamp {
    brightness: f32,
    tint: Color,
}

/// 램프 종류별 발광 세기 배율.
fn lamp_gain(kind: LampKind) -> f32 {
    match kind {
        LampKind::Led { .. } => 16.0,
        LampKind::Chance => 12.0,
        LampKind::ReelBacklight(_) => 0.33,
        LampKind::StopButton(_) => 5.0,
        LampKind::BetButton => 5.0,
        LampKind::Marquee => 1.9,
        LampKind::Belly => 1.0,
        LampKind::PayLine(_) => 11.0,
        LampKind::LineLamp(_) => 9.0,
        LampKind::BetOneButton => 5.0,
        LampKind::TopLamp => 9.0,
        LampKind::Status(_) => 3.2,
    }
}

/// 램프의 밝기와 색을 발광 재질로 반영한다.
pub(crate) fn apply_lamps(
    mut commands: Commands,
    settings: Res<Settings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lamps: Query<(
        Entity,
        &Lamp,
        &MeshMaterial3d<StandardMaterial>,
        Option<&mut AppliedLamp>,
    )>,
) {
    let led_scale = settings.config.graphics.led.brightness;
    for (entity, lamp, material, applied) in &mut lamps {
        let scale = match lamp.kind {
            // 릴 조명과 패널 백라이트는 가독성에 필요하므로 LED 밝기 설정의 영향을 덜 받는다.
            LampKind::ReelBacklight(_) | LampKind::Marquee | LampKind::Belly => {
                0.6 + 0.4 * led_scale.min(1.0)
            }
            _ => led_scale,
        };
        let brightness = lamp.brightness * scale;
        if let Some(applied) = &applied
            && (applied.brightness - brightness).abs() < 0.004
            && applied.tint == lamp.tint
        {
            continue;
        }
        let Some(mut asset) = materials.get_mut(&material.0) else {
            continue;
        };
        asset.emissive = lamp.tint.to_linear() * (brightness * lamp_gain(lamp.kind));
        let record = AppliedLamp {
            brightness,
            tint: lamp.tint,
        };
        match applied {
            Some(mut applied) => *applied = record,
            None => {
                commands.entity(entity).insert(record);
            }
        }
    }
}

/// 내부 관찰 모드에서는 외장을 숨긴다.
pub(crate) fn apply_view_mode(
    state: Res<State<AppState>>,
    view: Res<ViewMode>,
    mut shell: Query<&mut Visibility, With<ShellPart>>,
) {
    if !view.is_changed() && !state.is_changed() {
        return;
    }
    let visibility = if *state.get() == AppState::InGame && *view == ViewMode::Mechanical {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut part in &mut shell {
        *part = visibility;
    }
}

/// 투입구로 들어가는 메달 연출.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct InsertingMedal {
    elapsed: f32,
    duration: f32,
}

/// 메달 메시 (물리 계층과 공유).
#[derive(Resource, Debug, Clone)]
pub struct MedalMesh(pub Handle<Mesh>);

pub(crate) fn spawn_inserted_medals(
    mut commands: Commands,
    mut events: MessageReader<MachineEvent>,
    settings: Res<Settings>,
    library: Res<MaterialLibrary>,
    mesh: Res<MedalMesh>,
) {
    for MachineEvent(event) in events.read() {
        if !matches!(event, GameEvent::MedalInserted { .. }) {
            continue;
        }
        commands.spawn((
            Name::new("InsertingMedal"),
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(library.medal.clone()),
            Transform::from_translation(MEDAL_SLOT_POS + Vec3::Y * 0.07),
            InsertingMedal {
                elapsed: 0.0,
                duration: settings.config.machine.animation.medal_insert_ms as f32 / 1000.0,
            },
        ));
    }
}

pub(crate) fn animate_inserted_medals(
    mut commands: Commands,
    time: Res<Time>,
    mut medals: Query<(Entity, &mut InsertingMedal, &mut Transform)>,
) {
    for (entity, mut medal, mut transform) in &mut medals {
        medal.elapsed += time.delta_secs();
        let t = (medal.elapsed / medal.duration.max(0.001)).min(1.0);
        // 투입구 위에서 잠깐 머문 뒤 가속하며 떨어진다.
        let drop = if t < 0.35 {
            0.0
        } else {
            ((t - 0.35) / 0.65).powi(2)
        };
        let top = MEDAL_SLOT_POS.y + 0.02 + MEDAL_RADIUS + 0.03 * (1.0 - (t / 0.35).min(1.0));
        let bottom = MEDAL_SLOT_POS.y + 0.018 - MEDAL_RADIUS;
        transform.translation = Vec3::new(
            MEDAL_SLOT_POS.x,
            top + (bottom - top) * drop,
            MEDAL_SLOT_POS.z,
        );
        // 메달은 세워서(축이 Z) 투입구 홈에 맞춰 들어간다.
        transform.rotation =
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(t * 1.2);
        if t >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn medal_mesh() -> Cylinder {
    Cylinder::new(MEDAL_RADIUS, MEDAL_THICKNESS)
}

/// 물리 계층이 만든 메달에 메시와 재질을 입힌다.
pub(crate) fn dress_medals(
    mut commands: Commands,
    library: Res<MaterialLibrary>,
    mesh: Res<MedalMesh>,
    medals: Query<Entity, Added<Medal>>,
) {
    for entity in &medals {
        commands.entity(entity).insert((
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(library.medal.clone()),
        ));
    }
}

/// 당첨 라인 막대는 켜졌을 때만 보인다. 꺼진 막대가 릴 앞을 가리지 않게 한다.
pub(crate) fn apply_payline_visibility(
    view: Res<ViewMode>,
    mut bars: Query<(&Lamp, &mut Visibility), With<PayLineBar>>,
) {
    for (lamp, mut visibility) in &mut bars {
        let wanted = if lamp.brightness > 0.02 && *view != ViewMode::Mechanical {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}
