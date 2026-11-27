#![allow(clippy::unwrap_used, clippy::expect_used)]

//! 메달 물리의 헤드리스 검사: 받침을 뚫고 지나가는 메달이 없는지, 개수 상한이 지켜지는지.

use std::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use fruit_star_config::ConfigSet;
use fruit_star_machine::layout::*;
use fruit_star_machine::{AppState, Medal, MedalCollision, MedalPaidOut, Settings};
use fruit_star_physics::PhysicsPlugin;

fn app(max_medals: u32) -> App {
    let mut config = ConfigSet::default();
    config.graphics.render.max_medals = max_medals;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        TransformPlugin,
        AssetPlugin::default(),
    ))
    .init_asset::<Mesh>()
    .init_state::<AppState>()
    .add_message::<MedalPaidOut>()
    .add_message::<fruit_star_machine::MedalCollision>()
    .init_resource::<fruit_star_machine::DebugLevel>()
    .insert_resource(Settings {
        config,
        root: std::path::PathBuf::from("."),
    })
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .add_plugins(PhysicsPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    app
}

fn positions(app: &mut App) -> Vec<Vec3> {
    let mut query = app.world_mut().query_filtered::<&Transform, With<Medal>>();
    query.iter(app.world()).map(|t| t.translation).collect()
}

#[test]
fn medals_land_in_the_tray_without_tunnelling() {
    let mut app = app(60);
    // 15장 지급 4번: 70ms 간격으로 한 장씩 배출
    for frame in 0..(60 * 4) {
        if frame % 4 == 0 && frame < 60 * 4 {
            app.world_mut().write_message(MedalPaidOut { remaining: 0 });
        }
        app.update();
        for p in positions(&mut app) {
            assert!(p.is_finite(), "메달 위치가 발산했다: {p}");
            // 캐비닛 앞면 안쪽으로 파고들지 않는다
            assert!(p.z > FRONT_Z - 0.01, "앞면 관통: {p}");
            // 받침 영역 안에서는 바닥 아래로 내려가지 않는다
            let over_tray = p.x.abs() < TRAY_HALF_WIDTH && p.z < TRAY_FRONT_Z;
            if over_tray {
                assert!(p.y > TRAY_FLOOR_Y - 0.004, "받침 바닥 관통: {p}");
            }
            // 어디서든 방 바닥 아래로 떨어지지 않는다
            assert!(p.y > -0.01, "바닥 관통: {p}");
        }
    }
    // 가라앉을 시간을 준다
    for _ in 0..240 {
        app.update();
    }
    let settled = positions(&mut app);
    assert_eq!(settled.len(), 60);
    let in_tray = settled
        .iter()
        .filter(|p| {
            p.x.abs() < TRAY_HALF_WIDTH + 0.01
                && p.z > FRONT_Z - 0.01
                && p.z < TRAY_FRONT_Z + 0.01
                && p.y > TRAY_FLOOR_Y - 0.004
                && p.y < TRAY_FLOOR_Y + 0.12
        })
        .count();
    assert!(
        in_tray >= 54,
        "대부분의 메달이 받침에 남아야 한다 (받침 {in_tray}/60)"
    );
}

#[test]
fn payout_reports_both_tray_and_coin_impacts_before_the_medals_settle() {
    let mut app = app(40);
    let mut cursor = app
        .world()
        .resource::<Messages<MedalCollision>>()
        .get_cursor();
    let mut strongest = [0.0_f32; 2];
    for frame in 0..180 {
        if frame < 120 && frame % 4 == 0 {
            app.world_mut().write_message(MedalPaidOut { remaining: 0 });
        }
        app.update();
        for hit in cursor.read(app.world().resource::<Messages<MedalCollision>>()) {
            let level = &mut strongest[usize::from(hit.other_medal)];
            *level = level.max(hit.strength);
        }
    }
    assert!(strongest[0] > 0.4, "tray impact: {}", strongest[0]);
    assert!(strongest[1] > 0.2, "coin impact: {}", strongest[1]);
}

#[test]
fn medal_count_never_exceeds_the_limit_and_oldest_are_reused() {
    let mut app = app(10);
    for _ in 0..40 {
        app.world_mut().write_message(MedalPaidOut { remaining: 0 });
        app.update();
        assert!(positions(&mut app).len() <= 10);
    }
    assert_eq!(positions(&mut app).len(), 10);
    let mut query = app.world_mut().query::<&Medal>();
    let mut serials: Vec<u64> = query.iter(app.world()).map(|m| m.serial).collect();
    serials.sort_unstable();
    // 가장 최근에 배출된 10장만 남는다 (오래된 것부터 재사용)
    assert_eq!(serials, (30..40).collect::<Vec<u64>>());
}

#[test]
fn zero_limit_disables_medals() {
    let mut app = app(0);
    for _ in 0..5 {
        app.world_mut().write_message(MedalPaidOut { remaining: 0 });
        app.update();
    }
    assert!(positions(&mut app).is_empty());
}

#[test]
fn leaving_the_game_clears_medals() {
    let mut app = app(20);
    for _ in 0..8 {
        app.world_mut().write_message(MedalPaidOut { remaining: 0 });
        app.update();
    }
    assert_eq!(positions(&mut app).len(), 8);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    app.update();
    app.update();
    assert!(positions(&mut app).is_empty());
}
