//! 조명, 환경 반사, 주변 공간.

use bevy::light::{DirectionalLightShadowMap, GlobalAmbientLight, PointLightShadowMap};
use bevy::prelude::*;
use fruit_star_machine::layout::{CABINET_DEPTH, PEDESTAL_HEIGHT};
use fruit_star_machine::{AppState, Settings};

use crate::image_ops::environment_cubemap;
use crate::materials::MaterialLibrary;

/// 환경 반사용 HDR 큐브맵.
#[derive(Resource, Debug, Clone)]
pub struct EnvironmentMap(pub Handle<Image>);

fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65_535.0
}

/// 게임장 분위기의 환경 복사 휘도: 어두운 실내, 천장 조명 띠, 좌우 네온, 점점이 박힌 불빛.
fn parlor_radiance(d: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = d;
    // 바탕: 위는 어두운 남색, 아래는 더 어두운 바닥 반사
    let up = y * 0.5 + 0.5;
    let mut rgb = [0.010 + 0.020 * up, 0.010 + 0.018 * up, 0.016 + 0.034 * up];
    let add = |rgb: &mut [f32; 3], color: [f32; 3], amount: f32| {
        for c in 0..3 {
            rgb[c] += color[c] * amount;
        }
    };
    // 천장 조명 띠 (따뜻한 흰색)
    if y > 0.35 {
        let u = x / y;
        let v = z / y;
        let stripe = ((u * 1.6).rem_euclid(1.0) - 0.5).abs();
        // 가장자리를 부드럽게 해서 광택면에 비친 조명이 계단처럼 보이지 않게 한다.
        let edge = ((0.15 - stripe) / 0.08).clamp(0.0, 1.0);
        let fade = ((2.4 - v.abs()) / 0.5).clamp(0.0, 1.0);
        add(&mut rgb, [1.0, 0.92, 0.80], 5.5 * edge * edge * fade);
    }
    // 좌우의 넓은 네온 패널
    if y.abs() < 0.32 && z > -0.5 {
        if x < -0.72 {
            add(&mut rgb, [1.0, 0.25, 0.55], 2.2);
        } else if x > 0.72 {
            add(&mut rgb, [0.25, 0.75, 1.0], 2.2);
        }
    }
    // 플레이어 뒤쪽에 점점이 박힌 불빛 (크롬에 비치는 하이라이트)
    if z > 0.15 && y > -0.1 {
        let u = x.atan2(z) / 0.21;
        let v = y.asin() / 0.21;
        let (cu, cv) = (u.floor() as i32, v.floor() as i32);
        if hash(cu, cv) > 0.62 {
            let ou = 0.25 + 0.5 * hash(cu + 17, cv);
            let ov = 0.25 + 0.5 * hash(cu, cv + 31);
            let dist = ((u - cu as f32 - ou).powi(2) + (v - cv as f32 - ov).powi(2)).sqrt();
            if dist < 0.16 {
                let palette = [
                    [1.0, 0.85, 0.5],
                    [1.0, 0.35, 0.3],
                    [0.4, 0.8, 1.0],
                    [0.7, 1.0, 0.6],
                ];
                let color = palette[(hash(cu + 5, cv + 9) * 3.99) as usize];
                add(&mut rgb, color, 7.0);
            }
        }
    }
    rgb
}

pub(crate) fn setup_environment(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let cubemap = environment_cubemap(256, parlor_radiance);
    commands.insert_resource(EnvironmentMap(images.add(cubemap)));
    commands.insert_resource(ClearColor(Color::srgb(0.004, 0.004, 0.008)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.75, 0.8, 1.0),
        brightness: 18.0,
        ..default()
    });
}

pub(crate) fn spawn_lights(mut commands: Commands, settings: Res<Settings>) {
    let render = &settings.config.graphics.render;
    // 스포트라이트의 그림자 해상도는 방향광용 설정을 따르고, 점광원은 별도 설정을 쓴다.
    commands.insert_resource(DirectionalLightShadowMap {
        size: render.quality.shadow_map_size(),
    });
    commands.insert_resource(PointLightShadowMap {
        size: render.quality.shadow_map_size(),
    });
    // 주광: 플레이어 머리 위 앞쪽에서 비추는 스포트라이트
    commands.spawn((
        Name::new("KeyLight"),
        SpotLight {
            color: Color::srgb(1.0, 0.95, 0.88),
            intensity: 420_000.0,
            range: 9.0,
            radius: 0.08,
            shadow_maps_enabled: render.shadows,
            inner_angle: 0.35,
            outer_angle: 0.62,
            ..default()
        },
        Transform::from_xyz(0.55, 2.55, 1.75).looking_at(Vec3::new(0.0, 0.85, 0.05), Vec3::Y),
    ));
    // 보조광: 반대편 낮은 곳에서 그림자를 풀어 준다
    commands.spawn((
        Name::new("FillLight"),
        PointLight {
            color: Color::srgb(0.65, 0.78, 1.0),
            intensity: 38_000.0,
            range: 6.0,
            radius: 0.2,
            ..default()
        },
        Transform::from_xyz(-1.3, 1.25, 1.5),
    ));
    // 윤곽광: 좌우 뒤에서 캐비닛 가장자리를 살린다
    for (x, color) in [
        (-1.1, Color::srgb(1.0, 0.3, 0.55)),
        (1.1, Color::srgb(0.3, 0.75, 1.0)),
    ] {
        commands.spawn((
            Name::new("RimLight"),
            PointLight {
                color,
                intensity: 26_000.0,
                range: 4.0,
                radius: 0.1,
                ..default()
            },
            Transform::from_xyz(x, 1.5, 0.15),
        ));
    }
}

/// 바닥, 뒷벽, 옆자리 게임기 윤곽.
#[derive(Component)]
pub(crate) struct NeighborRow;

pub(crate) fn show_neighbors(
    state: Res<State<AppState>>,
    mut rows: Query<&mut Visibility, With<NeighborRow>>,
) {
    for mut visibility in &mut rows {
        *visibility = if *state.get() == AppState::InGame {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub(crate) fn spawn_room(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    library: &MaterialLibrary,
) {
    let room = commands
        .spawn((Name::new("Room"), Transform::IDENTITY, Visibility::Visible))
        .id();
    commands.spawn((
        Name::new("Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(library.floor.clone()),
        Transform::IDENTITY,
        ChildOf(room),
    ));
    let wall_z = -CABINET_DEPTH / 2.0 - 0.06;
    commands.spawn((
        Name::new("BackWall"),
        Mesh3d(meshes.add(Cuboid::new(14.0, 4.0, 0.1))),
        MeshMaterial3d(library.wall.clone()),
        Transform::from_xyz(0.0, 2.0, wall_z - 0.05),
        ChildOf(room),
    ));
    // 옆자리 게임기: 어두운 상자와 희미하게 빛나는 화면으로 줄지어 선 느낌만 준다
    let neighbors = commands
        .spawn((
            NeighborRow,
            Transform::IDENTITY,
            Visibility::Hidden,
            ChildOf(room),
        ))
        .id();
    for (index, x) in [-1.50f32, -0.75, 0.75, 1.50].into_iter().enumerate() {
        commands.spawn((
            Name::new("NeighborPedestal"),
            Mesh3d(meshes.add(Cuboid::new(0.60, PEDESTAL_HEIGHT, 0.46))),
            MeshMaterial3d(library.dark_grey.clone()),
            Transform::from_xyz(x, PEDESTAL_HEIGHT / 2.0, 0.0),
            ChildOf(neighbors),
        ));
        commands.spawn((
            Name::new("NeighborCabinet"),
            Mesh3d(meshes.add(Cuboid::new(0.52, 0.84, 0.36))),
            MeshMaterial3d(library.dark_grey.clone()),
            Transform::from_xyz(x, PEDESTAL_HEIGHT + 0.42, 0.0),
            ChildOf(neighbors),
        ));
        let hue = [200.0, 35.0, 300.0, 140.0][index];
        let glow = materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: Color::hsl(hue, 0.7, 0.4).to_linear() * 0.10,
            ..default()
        });
        commands.spawn((
            Name::new("NeighborScreen"),
            Mesh3d(meshes.add(Rectangle::new(0.28, 0.10))),
            MeshMaterial3d(glow),
            Transform::from_xyz(x, PEDESTAL_HEIGHT + 0.55, 0.1805),
            ChildOf(neighbors),
        ));
    }
}
