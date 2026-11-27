#![allow(clippy::unwrap_used, clippy::expect_used)]

//! 장기 시뮬레이션: 원장 정합성과 환수율(가상 메달 기준)을 확인한다.

mod common;

use common::*;
use fruit_star_core::{RandomSource, SplitMix64};
use fruit_star_game::{
    Allowed, Flag, GameController, GameEvent, GameOutcome, Phase, PlayMode, choose_stop, evaluate,
    windows_at,
};

/// 한 게임을 진행한다. `pick`은 릴별로 노릴 인덱스를 정한다 (`None`이면 지금 누른다).
fn play(
    game: &mut GameController,
    order: [usize; 3],
    mut pick: impl FnMut(&GameController, usize, [Option<usize>; 3]) -> Option<usize>,
) -> GameOutcome {
    if !game.bet_placed() {
        game.press_bet().unwrap();
    }
    game.pull_lever().unwrap();
    game.tick(0.4);
    let mut stops = [None; 3];
    for reel in order {
        if let Some(index) = pick(game, reel, stops) {
            aim(game, reel, index);
        }
        game.press_stop(reel).unwrap();
        stops[reel] = game.reels()[reel].target_index();
    }
    let mut outcome = None;
    for _ in 0..200 {
        game.tick(0.25);
        for event in game.drain_events() {
            if let GameEvent::GameCompleted(o) = event {
                assert!(outcome.is_none(), "한 게임에 정산은 한 번뿐이어야 한다");
                outcome = Some(*o);
            }
        }
        if game.phase() == Phase::Ready {
            break;
        }
    }
    outcome.expect("게임이 끝나야 한다")
}

fn check_invariants(game: &GameController, outcome: &GameOutcome, previous_balance: u32) {
    let ledger = game.ledger();
    // 잔액 = 투입 − 베팅 + 지급 (초기 잔액 0)
    assert_eq!(
        u64::from(ledger.balance()),
        ledger.total_inserted() + ledger.total_paid() - ledger.total_bet()
    );
    // 이 게임의 잔액 변화 = 지급 − 베팅(리플레이 게임이면 0) + 자동 충전분
    let bet: u32 = outcome.bet_entries.iter().map(|e| e.amount).sum();
    assert_eq!(outcome.bet_entries.is_empty(), outcome.replay_game);
    assert_eq!(outcome.balance_after, ledger.balance());
    let refill = (outcome.balance_after + bet)
        .checked_sub(previous_balance + outcome.payout)
        .expect("잔액이 설명되지 않는 만큼 줄었다");
    assert!(
        refill == 0 || refill >= 50,
        "설명되지 않는 잔액 증가 {refill}"
    );
    // 지급은 표시된 심볼의 평가 결과와 정확히 일치한다
    let profile = game.profile();
    let recomputed = fruit_star_game::evaluate_lines(
        profile,
        &windows_at(profile, outcome.stops),
        outcome.lines,
    );
    assert_eq!(recomputed.payout, outcome.payout);
    assert_eq!(recomputed.wins, outcome.wins);
    assert!(outcome.payout <= profile.max_payout);
    // 추첨 결과가 허용하지 않은 역할은 성립하지 않는다
    for win in &outcome.wins {
        let ok = match win.role.bonus_kind() {
            Some(_) => !outcome.in_bonus,
            None => outcome.flag == Flag::Small(win.role),
        };
        assert!(ok, "허용되지 않은 역할 성립: {outcome:?}");
    }
}

struct Report {
    games: u32,
    rtp: f64,
    bonuses: u32,
}

fn simulate(seed: u64, games: u32, aimed: bool) -> Report {
    let mut game = seeded(PlayMode::Practice, 0, seed);
    let mut player = SplitMix64::new(seed ^ 0xA5A5_5A5A);
    let orders = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut bonuses = 0;
    for _ in 0..games {
        let order = orders[player.below(6) as usize];
        let random_index = [player.below(21), player.below(21), player.below(21)];
        let previous = game.credit();
        let outcome = play(&mut game, order, |game, reel, stops| {
            if !aimed {
                return Some(random_index[reel] as usize);
            }
            // 연습 모드의 힌트를 보고, 허용된 역할을 가장 잘 성립시키는 위치를 노린다.
            let (flag, carried) = game.practice_hint().unwrap();
            let in_bonus = game.bonus().is_some();
            let allowed = Allowed::from_flag(flag, if in_bonus { None } else { carried });
            let profile = game.profile();
            let mut best = (0u32, random_index[reel] as usize);
            for pressed in 0..profile.symbol_count {
                let decision = choose_stop(profile, allowed, stops, reel, pressed);
                let mut next = stops;
                next[reel] = Some(decision.index);
                // 남은 릴은 최선으로 누른다고 보고 성립 가능한 최대 가치를 구한다.
                let value = best_value(profile, allowed, next);
                if value > best.0 {
                    best = (value, pressed);
                }
            }
            Some(best.1)
        });
        check_invariants(&game, &outcome, previous);
        if outcome.bonus_started.is_some() {
            bonuses += 1;
        }
    }
    let ledger = game.ledger();
    Report {
        games,
        rtp: ledger.total_paid() as f64 / ledger.total_bet() as f64,
        bonuses,
    }
}

/// 남은 릴을 최선으로 눌렀을 때 얻을 수 있는 최대 가치 (지급 + 보너스·리플레이 가산점).
fn best_value(
    profile: &fruit_star_game::MachineProfile,
    allowed: Allowed,
    stops: [Option<usize>; 3],
) -> u32 {
    match stops.iter().position(Option::is_none) {
        None => {
            let s = [stops[0].unwrap(), stops[1].unwrap(), stops[2].unwrap()];
            let result = evaluate(profile, &windows_at(profile, s));
            result.payout + u32::from(result.replay) * 3 + u32::from(result.bonus.is_some()) * 100
        }
        Some(reel) => (0..profile.symbol_count)
            .map(|pressed| {
                let decision = choose_stop(profile, allowed, stops, reel, pressed);
                let mut next = stops;
                next[reel] = Some(decision.index);
                best_value(profile, allowed, next)
            })
            .max()
            .unwrap_or(0),
    }
}

#[test]
fn random_player_keeps_ledger_consistent_and_rtp_in_band() {
    let report = simulate(20_261_125, 60_000, false);
    println!(
        "무작위 누름: {}게임, 환수율 {:.1}%, 보너스 {}회",
        report.games,
        report.rtp * 100.0,
        report.bonuses
    );
    assert!(
        report.rtp > 0.45 && report.rtp < 1.0,
        "환수율 {}",
        report.rtp
    );
}

#[test]
fn aiming_player_gets_a_fair_but_not_runaway_return() {
    let report = simulate(77, 4_000, true);
    println!(
        "노려 누름: {}게임, 환수율 {:.1}%, 보너스 {}회",
        report.games,
        report.rtp * 100.0,
        report.bonuses
    );
    assert!(report.bonuses > 0);
    assert!(
        report.rtp > 0.80 && report.rtp < 1.10,
        "환수율 {}",
        report.rtp
    );
}
