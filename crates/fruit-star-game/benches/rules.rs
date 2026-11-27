//! 도메인 로직 벤치마크: 릴 제어, 조합 평가, 릴 운동, 한 게임 전체.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use fruit_star_core::{Reel, ReelTiming, SplitMix64};
use fruit_star_game::{
    Allowed, BonusKind, GameController, MachineProfile, Phase, PlayMode, Role, choose_stop,
    evaluate, windows_at,
};

fn bench_rules(c: &mut Criterion) {
    let profile = MachineProfile::fruit_classic();

    c.bench_function("evaluate (5 paylines)", |b| {
        let windows = windows_at(&profile, [3, 7, 11]);
        b.iter(|| evaluate(black_box(&profile), black_box(&windows)));
    });

    // 정지 버튼을 누르는 순간에 실행되는 계산. 첫 번째 정지가 두 수 앞까지 보므로 가장 무겁다.
    let allowed = Allowed {
        small: Some(Role::Grape),
        bonus: Some(BonusKind::Big),
    };
    c.bench_function("choose_stop: first reel (lookahead)", |b| {
        b.iter(|| choose_stop(black_box(&profile), allowed, [None; 3], 0, black_box(5)));
    });
    c.bench_function("choose_stop: second reel", |b| {
        b.iter(|| {
            choose_stop(
                black_box(&profile),
                allowed,
                [Some(4), None, None],
                1,
                black_box(9),
            )
        });
    });
    c.bench_function("choose_stop: last reel", |b| {
        b.iter(|| {
            choose_stop(
                black_box(&profile),
                allowed,
                [Some(4), Some(5), None],
                2,
                black_box(13),
            )
        });
    });

    c.bench_function("reel update (one frame)", |b| {
        let Ok(mut reel) = Reel::new(0, profile.strips[0].clone(), ReelTiming::default()) else {
            return;
        };
        if reel.start().is_err() {
            return;
        }
        b.iter(|| reel.update(black_box(1.0 / 120.0)));
    });

    c.bench_function("full game (bet → stop → settle)", |b| {
        let Ok(mut game) = GameController::new(
            profile.clone(),
            ReelTiming::default(),
            PlayMode::FreePlay,
            0,
            9_999,
            1,
            Box::new(SplitMix64::new(7)),
        ) else {
            return;
        };
        b.iter(|| {
            if !game.bet_placed() {
                let _ = game.press_bet();
            }
            let _ = game.pull_lever();
            game.tick(0.5);
            for reel in 0..3 {
                let _ = game.press_stop(reel);
            }
            while game.phase() != Phase::Ready {
                game.tick(0.1);
            }
            black_box(game.drain_events())
        });
    });
}

criterion_group!(benches, bench_rules);
criterion_main!(benches);
