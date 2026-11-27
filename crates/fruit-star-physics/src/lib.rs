//! FRUIT STAR 3D 물리 계층: 배출된 메달이 받침으로 떨어져 구르고 부딪히는 연출.
//!
//! 메달 수는 상한이 있고, 상한에 이르면 가장 오래된 메달을 재사용한다(오브젝트 풀).
//! 물리는 고정 시간 간격으로 갱신하며, 얇은 메달이 받침을 뚫지 않도록 연속 충돌
//! 검사(CCD)와 두꺼운 충돌체를 쓴다. 게임 결과에는 전혀 영향을 주지 않는다.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use fruit_star_core::{RandomSource, SplitMix64};
use fruit_star_machine::layout::*;
use fruit_star_machine::{AppState, Medal, MedalCollision, MedalPaidOut, Settings};

/// 물리 갱신 주기 (Hz).
pub const PHYSICS_HZ: f64 = 120.0;

/// 물리 플러그인.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            RapierPhysicsPlugin::<NoUserData>::default()
                // 메달 크기(지름 2.5cm)에 맞춰 허용 오차를 줄인다.
                .with_length_unit(MEDAL_RADIUS * 2.0)
                .in_fixed_schedule(),
        )
        .insert_resource(Time::<Fixed>::from_hz(PHYSICS_HZ))
        .insert_resource(TimestepMode::Fixed {
            dt: (1.0 / PHYSICS_HZ) as f32,
            substeps: 2,
        })
        .init_resource::<MedalPool>()
        .add_systems(Startup, spawn_colliders)
        .add_systems(
            Update,
            (spawn_medals, contain_medals).run_if(in_state(AppState::InGame)),
        )
        .add_systems(
            FixedUpdate,
            report_collisions
                .after(PhysicsSet::StepSimulation)
                .before(PhysicsSet::Writeback)
                .run_if(in_state(AppState::InGame)),
        )
        .add_systems(OnExit(AppState::InGame), clear_medals);

        // 충돌 영역 표시는 개발 빌드에서, 기즈모를 그릴 수 있는 환경(창이 있는 실행)일 때만 붙인다.
        #[cfg(debug_assertions)]
        if app.is_plugin_added::<bevy::gizmos::GizmoPlugin>() {
            app.add_plugins(RapierDebugRenderPlugin {
                enabled: false,
                ..default()
            })
            .add_systems(Update, apply_debug_render);
        }
    }
}

/// 디버그 단계 2에서 충돌 영역을 그린다 (개발 빌드 전용).
#[cfg(debug_assertions)]
fn apply_debug_render(
    level: Res<fruit_star_machine::DebugLevel>,
    mut context: ResMut<DebugRenderContext>,
) {
    let wanted = level.0 >= 2;
    if context.enabled != wanted {
        context.enabled = wanted;
    }
}

/// 메달 풀 상태.
#[derive(Resource, Debug)]
pub struct MedalPool {
    next_serial: u64,
    rng: SplitMix64,
}

impl Default for MedalPool {
    fn default() -> Self {
        Self {
            next_serial: 0,
            // 연출용 흩뿌림이므로 고정 시드로 충분하다.
            rng: SplitMix64::new(0x5EED_CAFE),
        }
    }
}

impl MedalPool {
    /// −1 ~ 1 범위의 난수.
    fn jitter(&mut self) -> f32 {
        self.rng.below(20_001) as f32 / 10_000.0 - 1.0
    }
}

/// 고정 충돌체: 받침, 캐비닛 앞면, 바닥.
fn spawn_colliders(mut commands: Commands) {
    let depth = TRAY_FRONT_Z - FRONT_Z;
    let mid_z = (TRAY_FRONT_Z + FRONT_Z) / 2.0;
    // 얇은 판 대신 두꺼운 상자를 써서 빠른 메달이 뚫고 지나가지 못하게 한다.
    let thick = 0.06;
    let wall_height = TRAY_LIP_HEIGHT - 0.006;
    let boxes = [
        // 받침 바닥 (윗면이 TRAY_FLOOR_Y)
        (
            "TrayFloorCollider",
            Vec3::new(TRAY_HALF_WIDTH + thick, thick / 2.0, depth / 2.0 + thick),
            Vec3::new(0.0, TRAY_FLOOR_Y - thick / 2.0, mid_z),
        ),
        // 앞 턱
        (
            "TrayLipCollider",
            Vec3::new(TRAY_HALF_WIDTH + thick, wall_height / 2.0, thick / 2.0),
            Vec3::new(
                0.0,
                TRAY_FLOOR_Y + wall_height / 2.0,
                TRAY_FRONT_Z + thick / 2.0,
            ),
        ),
        // 좌우 벽
        (
            "TraySideCollider",
            Vec3::new(thick / 2.0, wall_height / 2.0, depth / 2.0),
            Vec3::new(
                -(TRAY_HALF_WIDTH + thick / 2.0),
                TRAY_FLOOR_Y + wall_height / 2.0,
                mid_z,
            ),
        ),
        (
            "TraySideCollider",
            Vec3::new(thick / 2.0, wall_height / 2.0, depth / 2.0),
            Vec3::new(
                TRAY_HALF_WIDTH + thick / 2.0,
                TRAY_FLOOR_Y + wall_height / 2.0,
                mid_z,
            ),
        ),
        // 캐비닛 앞면 (배출구 뒤쪽 벽)
        (
            "CabinetFrontCollider",
            Vec3::new(CABINET_WIDTH / 2.0, CABINET_HEIGHT / 2.0, thick / 2.0),
            Vec3::new(
                0.0,
                CABINET_BOTTOM + CABINET_HEIGHT / 2.0,
                FRONT_Z - thick / 2.0,
            ),
        ),
        // 받침대 윗면과 방 바닥 (받침을 넘친 메달이 떨어질 곳)
        (
            "PedestalCollider",
            Vec3::new(
                PEDESTAL_SIZE.x / 2.0,
                PEDESTAL_SIZE.y / 2.0,
                PEDESTAL_SIZE.z / 2.0,
            ),
            Vec3::new(0.0, PEDESTAL_SIZE.y / 2.0, 0.0),
        ),
        (
            "FloorCollider",
            Vec3::new(7.0, 0.25, 7.0),
            Vec3::new(0.0, -0.25, 0.0),
        ),
    ];
    for (name, half, center) in boxes {
        commands.spawn((
            Name::new(name),
            RigidBody::Fixed,
            Collider::cuboid(half.x, half.y, half.z),
            Friction::coefficient(0.55),
            Restitution::coefficient(0.10),
            Transform::from_translation(center),
        ));
    }
}

/// 배출구에서 나오는 메달의 초기 자세와 속도.
fn launch(pool: &mut MedalPool) -> (Transform, Velocity) {
    let position = CHUTE_POS
        + Vec3::new(
            pool.jitter() * 0.038,
            pool.jitter() * 0.006,
            MEDAL_RADIUS * 0.6,
        );
    // 메달은 거의 눕다시피 한 자세로 미끄러져 나온다. 세워서 내보내면 굴러서 받침을 넘는다.
    let rotation = Quat::from_euler(
        EulerRot::XYZ,
        0.35 + pool.jitter() * 0.25,
        pool.jitter() * 3.0,
        pool.jitter() * 0.25,
    );
    let velocity = Velocity {
        linear: Vec3::new(pool.jitter() * 0.10, -0.05, 0.20 + pool.jitter() * 0.06),
        angular: Vec3::new(
            1.5 + pool.jitter() * 1.5,
            pool.jitter() * 4.0,
            pool.jitter() * 2.0,
        ),
    };
    (
        Transform::from_translation(position).with_rotation(rotation),
        velocity,
    )
}

/// 메달이 배출될 때마다 하나씩 떨어뜨린다. 상한에 이르면 가장 오래된 메달을 재사용한다.
fn spawn_medals(
    mut commands: Commands,
    mut paid: MessageReader<MedalPaidOut>,
    settings: Res<Settings>,
    mut pool: ResMut<MedalPool>,
    mut medals: Query<(Entity, &mut Medal, &mut Transform, &mut Velocity)>,
) {
    let limit = settings.config.graphics.render.max_medals as usize;
    let mut alive = medals.iter().count();
    for _ in paid.read() {
        if limit == 0 {
            continue;
        }
        let (transform, velocity) = launch(&mut pool);
        let serial = pool.next_serial;
        pool.next_serial += 1;
        if alive >= limit {
            let oldest = medals
                .iter_mut()
                .min_by_key(|(_, medal, _, _)| medal.serial);
            if let Some((entity, mut medal, mut old_transform, mut old_velocity)) = oldest {
                medal.serial = serial;
                *old_transform = transform;
                *old_velocity = velocity;
                // 잠들어 있던 메달을 깨운다.
                commands.entity(entity).insert(Sleeping::default());
            }
            continue;
        }
        alive += 1;
        commands.spawn((
            Name::new("Medal"),
            Medal { serial },
            RigidBody::Dynamic,
            // 원기둥 축은 Y (메달 면이 위아래)
            Collider::cylinder(MEDAL_THICKNESS / 2.0, MEDAL_RADIUS),
            ColliderMassProperties::Density(7_500.0),
            Friction::coefficient(0.45),
            Restitution::coefficient(0.12),
            Damping {
                linear_damping: 0.35,
                angular_damping: 1.6,
            },
            Ccd::enabled(),
            ActiveEvents::COLLISION_EVENTS,
            velocity,
            transform,
        ));
    }
}

/// 메달 충돌을 음향 계층에 알린다.
fn report_collisions(
    mut events: MessageReader<CollisionEvent>,
    medals: Query<(&Transform, &Velocity), With<Medal>>,
    mut collisions: MessageWriter<MedalCollision>,
) {
    for event in events.read() {
        let CollisionEvent::Started(a, b, _) = event else {
            continue;
        };
        // Writeback 전의 속도로 부딪히기 직전 두 물체의 상대 운동을 잰다.
        let (position, speed, other_medal) = match (medals.get(*a), medals.get(*b)) {
            (Ok((ta, va)), Ok((tb, vb))) => (
                ta.translation.midpoint(tb.translation),
                (va.linear - vb.linear).length(),
                true,
            ),
            (Ok((t, v)), Err(_)) | (Err(_), Ok((t, v))) => {
                (t.translation, v.linear.length(), false)
            }
            _ => continue,
        };
        // 이 장면의 Rapier 길이 단위는 메달 지름이다. 배출구에서 받침까지
        // 자유 낙하한 속도를 기준으로 삼아 연출 속도에서도 첫 충격을 살린다.
        let gravity = 9.81 * (2.0 * MEDAL_RADIUS);
        let drop_speed = (2.0 * gravity * (CHUTE_POS.y - TRAY_FLOOR_Y)).sqrt();
        let strength = (speed / drop_speed).min(1.0);
        if strength < 0.18 {
            continue;
        }
        collisions.write(MedalCollision {
            position,
            other_medal,
            strength,
        });
    }
}

/// 수치 오류 등으로 세계 밖으로 나간 메달을 정리한다.
fn contain_medals(mut commands: Commands, medals: Query<(Entity, &Transform), With<Medal>>) {
    for (entity, transform) in &medals {
        let p = transform.translation;
        if !p.is_finite() || p.y < -1.0 || p.length() > 8.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn clear_medals(mut commands: Commands, medals: Query<Entity, With<Medal>>) {
    for entity in &medals {
        commands.entity(entity).despawn();
    }
}
