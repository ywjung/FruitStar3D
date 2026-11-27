#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fruit_star_core::{CreditError, GameState, LedgerEntryKind, ReelState, Symbol};
use fruit_star_game::{
    BonusKind, Flag, GameError, GameEvent, MachineProfile, Phase, PlayMode, Role,
};

fn profile() -> MachineProfile {
    MachineProfile::fruit_classic()
}

/// 릴 `reel`에서 `symbol`이 가운데 줄에 오는 인덱스.
fn centre(reel: usize, symbol: Symbol) -> usize {
    profile().strips[reel]
        .iter()
        .position(|s| *s == symbol)
        .unwrap()
}

fn aim_all(symbol: Symbol) -> [(usize, usize); 3] {
    [
        (0, centre(0, symbol)),
        (1, centre(1, symbol)),
        (2, centre(2, symbol)),
    ]
}

fn count(events: &[GameEvent], pred: impl Fn(&GameEvent) -> bool) -> usize {
    events.iter().filter(|e| pred(e)).count()
}

#[test]
fn a_full_blank_game_follows_the_expected_flow() {
    // 큰 추첨 값 → 꽝
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[65_535]));
    assert_eq!(game.phase(), Phase::Ready);
    assert_eq!(game.game_state(), GameState::Ready);

    // 베팅 없이 레버를 당기면 거부된다
    assert_eq!(game.pull_lever(), Err(GameError::BetRequired));

    game.press_bet().unwrap();
    assert_eq!(game.credit(), 47);
    assert_eq!(game.press_bet(), Err(GameError::AlreadyBet));
    assert_eq!(game.credit(), 47);

    game.pull_lever().unwrap();
    assert_eq!(game.phase(), Phase::Spinning);
    assert_eq!(game.game_state(), GameState::Spinning);
    let events = game.drain_events();
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::LeverPulled { .. })),
        1
    );
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::ReelStarted(_))),
        3
    );

    // 회전 중에는 베팅·레버 입력을 받지 않는다
    assert!(matches!(
        game.press_bet(),
        Err(GameError::WrongPhase { .. })
    ));
    assert!(matches!(
        game.pull_lever(),
        Err(GameError::WrongPhase { .. })
    ));

    // 정속에 이르기 전에는 정지 입력을 받지 않는다
    assert!(!game.can_stop(0));
    assert_eq!(game.press_stop(0), Err(GameError::StopNotReady { reel: 0 }));
    spin_up(&mut game);
    assert!(game.can_stop(0));

    game.press_stop(1).unwrap();
    assert_eq!(game.phase(), Phase::Stopping);
    assert_eq!(game.game_state(), GameState::Stopping);
    // 중복 정지 입력은 무시된다
    assert_eq!(game.press_stop(1), Err(GameError::StopNotReady { reel: 1 }));
    assert_eq!(game.press_stop(3), Err(GameError::InvalidReel { reel: 3 }));
    game.press_stop(0).unwrap();
    game.press_stop(2).unwrap();

    let events = finish(&mut game);
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::StopPressed(_))),
        3
    );
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::ReelStopped { .. })),
        3
    );
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::GameCompleted(_))),
        1
    );
    let outcome = outcome_of(&events);
    assert_eq!(outcome.game_id, 1);
    assert_eq!(outcome.flag, Flag::Blank);
    assert!(outcome.wins.is_empty());
    assert_eq!(outcome.payout, 0);
    assert_eq!(outcome.stop_order, [1, 0, 2]);
    assert_eq!(outcome.balance_after, 47);
    assert_eq!(outcome.bet_entries.len(), 1);
    assert_eq!(outcome.bet_entries[0].kind, LedgerEntryKind::Bet);
    assert_eq!(outcome.bet, 3);
    assert_eq!(outcome.lines, 5);
    assert!(outcome.payout_entry.is_none());
    assert_eq!(game.credit(), 47);
    assert_eq!(game.phase(), Phase::Ready);
    assert!(!game.bet_placed());
    assert_eq!(game.next_game_id(), 2);
}

#[test]
fn displayed_symbols_match_the_internal_result() {
    let mut game = seeded(PlayMode::FreePlay, 0, 7);
    for _ in 0..300 {
        if !game.bet_placed() {
            game.press_bet().unwrap();
        }
        game.pull_lever().unwrap();
        spin_up(&mut game);
        for reel in [2, 0, 1] {
            game.tick(0.013 * (reel as f64 + 1.0));
            game.press_stop(reel).unwrap();
        }
        let events = finish(&mut game);
        let outcome = outcome_of(&events);
        for reel in 0..3 {
            let r = &game.reels()[reel];
            assert_eq!(r.state(), ReelState::Stopped);
            // 릴이 실제로 멈춘 위치 = 기록된 정지 위치 = 평가에 쓰인 심볼
            assert_eq!(r.current_index(), outcome.stops[reel]);
            assert_eq!(r.visible(), outcome.windows[reel]);
            assert_eq!(r.position(), outcome.stops[reel] as f64);
            assert!(outcome.slips[reel] <= game.profile().max_slip);
        }
        // ReelStopped 이벤트의 인덱스도 일치한다
        for event in &events {
            if let GameEvent::ReelStopped { reel, index } = event {
                assert_eq!(*index, outcome.stops[*reel]);
            }
        }
    }
}

#[test]
fn grape_pays_exactly_once() {
    let p = profile();
    let roll = roll_for(&p, Flag::Small(Role::Grape));
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[roll]));
    let (outcome, events) = play_aimed(&mut game, aim_all(Symbol::Bell));
    assert_eq!(outcome.flag, Flag::Small(Role::Grape));
    assert!(outcome.wins.iter().all(|w| w.role == Role::Grape));
    assert!(!outcome.wins.is_empty(), "포도는 어디서 눌러도 성립한다");
    assert_eq!(outcome.payout, 8);
    assert_eq!(game.credit(), 50 - 3 + 8);
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::MedalDispensed(8))),
        1
    );
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::GameCompleted(_))),
        1
    );
    let entry = outcome.payout_entry.unwrap();
    assert_eq!(entry.kind, LedgerEntryKind::Payout);
    assert_eq!(entry.amount, 8);
    assert_eq!(entry.game_id, Some(1));
    // 결과 표시 이후에도 추가 지급은 없다
    for _ in 0..200 {
        game.tick(0.05);
    }
    assert!(game.drain_events().is_empty());
    assert_eq!(game.credit(), 55);
    assert_eq!(game.ledger().total_paid(), 8);
}

#[test]
fn result_phase_lasts_for_the_payout_duration() {
    let p = profile();
    let roll = roll_for(&p, Flag::Small(Role::Grape));
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[roll]));
    game.press_bet().unwrap();
    game.pull_lever().unwrap();
    spin_up(&mut game);
    for reel in 0..3 {
        game.press_stop(reel).unwrap();
    }
    while game.phase() != Phase::Result {
        game.tick(0.01);
    }
    // 결과 표시 중에는 다음 게임을 시작할 수 없다
    assert!(matches!(
        game.press_bet(),
        Err(GameError::WrongPhase { .. })
    ));
    let expected = p.flow.result_hold_s + 8.0 * p.flow.payout_interval_s;
    let mut elapsed = 0.0;
    while game.phase() == Phase::Result {
        game.tick(0.01);
        elapsed += 0.01;
    }
    assert!(
        (elapsed - expected).abs() <= 0.02,
        "{elapsed} vs {expected}"
    );
}

#[test]
fn replay_gives_a_free_next_game() {
    let p = profile();
    let roll = roll_for(&p, Flag::Small(Role::Replay));
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[roll, 65_535]));
    let (outcome, events) = play_aimed(&mut game, aim_all(Symbol::Star));
    assert!(outcome.replay_awarded);
    assert_eq!(outcome.payout, 0);
    assert_eq!(game.credit(), 47);
    // 리플레이 자동 베팅
    assert!(game.bet_placed());
    assert!(game.replay_pending());
    assert_eq!(
        count(&events, |e| matches!(
            e,
            GameEvent::BetPressed { replay: true, .. }
        )),
        1
    );
    assert_eq!(game.press_bet(), Err(GameError::AlreadyBet));

    // 다음 게임은 메달 차감 없이 진행된다
    let (next, _) = play_aimed(&mut game, aim_all(Symbol::Star));
    assert!(next.replay_game);
    assert!(next.bet_entries.is_empty());
    assert_eq!(next.game_id, 2);
    assert_eq!(game.credit(), 47);
    assert_eq!(game.ledger().total_bet(), 3);
    assert!(!game.bet_placed());
}

#[test]
fn classic_mode_rejects_bet_without_credit_and_never_goes_negative() {
    let mut game = controller_with(PlayMode::Classic, 2, Scripted::rolls(&[]));
    assert_eq!(
        game.press_bet(),
        Err(GameError::Credit(CreditError::Insufficient {
            need: 3,
            have: 2
        }))
    );
    assert_eq!(game.credit(), 2);
    assert!(!game.bet_placed());
    game.insert_medals(1).unwrap();
    game.press_bet().unwrap();
    assert_eq!(game.credit(), 0);
    let events = game.drain_events();
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::MedalInserted { .. })),
        1
    );
}

#[test]
fn free_play_refills_automatically() {
    let mut game = controller_with(PlayMode::FreePlay, 0, Scripted::rolls(&[]));
    game.press_bet().unwrap();
    assert!(game.credit() >= 47);
    let events = game.drain_events();
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::MedalInserted { .. })),
        1
    );
}

#[test]
fn bonus_flag_is_carried_until_aligned_then_bonus_runs_to_its_cap() {
    let p = profile();
    let big = roll_for(&p, Flag::Bonus(BonusKind::Big));
    // 1게임: 보너스 당첨(램프 추첨 65535 → 선고지 없음), 2게임: 꽝(이월 중), 이후: 보너스 게임은 모두 포도(값 0)
    let mut rolls = vec![big, LAMP_AFTER_REELS, 65_535];
    rolls.extend(std::iter::repeat_n(0, 200));
    let mut game = controller_with(PlayMode::Classic, 500, Scripted::rolls(&rolls));
    game.power_on();

    // 1게임: 레버를 당겨 보너스에 당첨되어도, 후고지면 릴이 도는 동안에는 램프가 켜지지 않는다
    game.press_bet().unwrap();
    game.pull_lever().unwrap();
    spin_up(&mut game);
    assert!(
        !game.chance_lamp(),
        "게임이 끝나기 전에 당첨을 드러내면 안 된다"
    );
    for (reel, index) in aim_all(Symbol::Bell) {
        aim(&mut game, reel, index);
        game.press_stop(reel).unwrap();
    }
    let events = finish(&mut game);
    let first = outcome_of(&events);
    // 7을 노리지 않아 맞추지 못했다 → 게임이 끝난 뒤 램프 점등
    assert_eq!(first.flag, Flag::Bonus(BonusKind::Big));
    assert!(first.wins.is_empty());
    assert!(game.chance_lamp());
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::ChanceLampChanged(true))),
        1
    );
    assert!(game.bonus().is_none());

    // 2게임: 빨간 7을 노려 맞춘다 → 보너스 시작, 램프 소등
    let (second, events) = play_aimed(&mut game, aim_all(Symbol::RedSeven));
    assert_eq!(second.flag, Flag::Blank);
    assert_eq!(second.bonus_started, Some(BonusKind::Big));
    assert!(second.wins.iter().any(|w| w.role == Role::BigRed));
    assert!(!game.chance_lamp());
    assert_eq!(
        count(&events, |e| matches!(
            e,
            GameEvent::BonusStarted(BonusKind::Big)
        )),
        1
    );
    assert_eq!(
        count(&events, |e| matches!(
            e,
            GameEvent::ChanceLampChanged(false)
        )),
        1
    );
    let bonus = *game.bonus().unwrap();
    assert_eq!(bonus.payout_cap, p.bonus.big_payout_cap);
    assert_eq!(game.game_state(), GameState::Bonus);

    // 보너스 게임: 지급 누계가 상한에 이르면 끝난다
    let mut completed = None;
    let mut bonus_games = 0;
    let mut bonus_paid = 0;
    while game.bonus().is_some() {
        let (outcome, events) = play_aimed(&mut game, aim_all(Symbol::Bell));
        assert!(outcome.in_bonus);
        assert_eq!(outcome.payout, 8, "보너스 게임에서는 포도가 항상 성립한다");
        bonus_games += 1;
        bonus_paid += outcome.payout;
        for event in events {
            if let GameEvent::BonusCompleted { kind, paid, games } = event {
                completed = Some((kind, paid, games));
            }
        }
        assert!(bonus_games < 100);
    }
    let (kind, paid, games) = completed.expect("BonusCompleted 이벤트가 있어야 한다");
    assert_eq!(kind, BonusKind::Big);
    assert_eq!(paid, bonus_paid);
    assert_eq!(games, bonus_games);
    assert!(paid >= p.bonus.big_payout_cap);
    assert!(paid < p.bonus.big_payout_cap + p.max_payout);
    assert_eq!(game.game_state(), GameState::Ready);
}

/// 램프 추첨 값(65536 기준). `below(100)`에서 각각 0(선고지)과 99(후고지)가 된다.
const LAMP_ON_LEVER: u32 = 0;
const LAMP_AFTER_REELS: u32 = 65_535;

#[test]
fn some_bonus_wins_light_the_lamp_when_the_lever_is_pulled() {
    let p = profile();
    assert_eq!(p.bonus.lamp_lead_percent, 25);
    let big = roll_for(&p, Flag::Bonus(BonusKind::Big));
    let mut game = controller_with(
        PlayMode::Classic,
        500,
        Scripted::rolls(&[big, LAMP_ON_LEVER, 65_535]),
    );

    // 레버를 당기는 순간 켜진다 (릴이 돌기 전)
    game.press_bet().unwrap();
    game.pull_lever().unwrap();
    assert!(game.chance_lamp());
    let events = game.drain_events();
    let lever = events
        .iter()
        .position(|e| matches!(e, GameEvent::LeverPulled { .. }))
        .unwrap();
    let lamp = events
        .iter()
        .position(|e| matches!(e, GameEvent::ChanceLampChanged(true)))
        .unwrap();
    let reel = events
        .iter()
        .position(|e| matches!(e, GameEvent::ReelStarted(_)))
        .unwrap();
    assert!(lever < lamp && lamp < reel, "{events:?}");

    // 맞추지 못하고 끝나도 다시 켜는 이벤트는 없다 (효과음이 두 번 나지 않는다)
    spin_up(&mut game);
    for (reel, index) in aim_all(Symbol::Bell) {
        aim(&mut game, reel, index);
        game.press_stop(reel).unwrap();
    }
    let events = finish(&mut game);
    assert!(game.chance_lamp());
    assert_eq!(
        count(&events, |e| matches!(e, GameEvent::ChanceLampChanged(_))),
        0
    );

    // 다음 게임에 맞추면 꺼지고 보너스가 시작된다
    let (outcome, events) = play_aimed(&mut game, aim_all(Symbol::RedSeven));
    assert_eq!(outcome.bonus_started, Some(BonusKind::Big));
    assert!(!game.chance_lamp());
    assert_eq!(
        count(&events, |e| matches!(
            e,
            GameEvent::ChanceLampChanged(false)
        )),
        1
    );
}

#[test]
fn a_lamp_lit_on_the_lever_goes_out_when_the_bonus_is_aligned_in_the_same_game() {
    let p = profile();
    let big = roll_for(&p, Flag::Bonus(BonusKind::Big));
    let mut game = controller_with(
        PlayMode::Classic,
        500,
        Scripted::rolls(&[big, LAMP_ON_LEVER]),
    );
    game.press_bet().unwrap();
    game.pull_lever().unwrap();
    assert!(game.chance_lamp());
    game.drain_events();
    spin_up(&mut game);
    for (reel, index) in aim_all(Symbol::RedSeven) {
        aim(&mut game, reel, index);
        game.press_stop(reel).unwrap();
    }
    let events = finish(&mut game);
    assert_eq!(outcome_of(&events).bonus_started, Some(BonusKind::Big));
    assert!(!game.chance_lamp());
    assert_eq!(
        count(&events, |e| matches!(
            e,
            GameEvent::ChanceLampChanged(false)
        )),
        1
    );
}

#[test]
fn regular_bonus_starts_from_bar() {
    let p = profile();
    let regular = roll_for(&p, Flag::Bonus(BonusKind::Regular));
    let mut game = controller_with(PlayMode::Classic, 100, Scripted::rolls(&[regular]));
    let (outcome, _) = play_aimed(&mut game, aim_all(Symbol::Bar));
    assert_eq!(outcome.bonus_started, Some(BonusKind::Regular));
    assert_eq!(game.bonus().unwrap().payout_cap, p.bonus.regular_payout_cap);
    // 당첨된 게임에서 바로 맞추면 램프는 켜지지 않는다
    assert!(!game.chance_lamp());
}

#[test]
fn sevens_cannot_be_aligned_without_the_bonus_flag() {
    let mut game = controller_with(PlayMode::Classic, 100, Scripted::rolls(&[65_535]));
    let (outcome, _) = play_aimed(&mut game, aim_all(Symbol::RedSeven));
    assert!(outcome.wins.is_empty());
    assert_eq!(outcome.bonus_started, None);
    assert!(
        outcome.slips.iter().any(|&s| s > 0),
        "릴 제어가 7을 피해야 한다"
    );
}

#[test]
fn practice_mode_exposes_the_flag_and_allows_forcing_it() {
    let mut game = controller_with(PlayMode::Practice, 0, Scripted::rolls(&[65_535, 65_535]));
    assert_eq!(game.practice_hint(), Some((Flag::Blank, None)));
    game.set_forced_flag(Some(Flag::Small(Role::Melon)));
    game.press_bet().unwrap();
    game.pull_lever().unwrap();
    assert_eq!(game.practice_hint(), Some((Flag::Small(Role::Melon), None)));
    spin_up(&mut game);
    for (reel, index) in aim_all(Symbol::Melon) {
        aim(&mut game, reel, index);
        game.press_stop(reel).unwrap();
    }
    let outcome = outcome_of(&finish(&mut game));
    assert_eq!(outcome.payout, 12);

    // 강제 지정은 한 게임에만 적용된다
    let (next, _) = play_aimed(&mut game, aim_all(Symbol::Melon));
    assert_eq!(next.flag, Flag::Blank);

    // 다른 모드에서는 힌트도 강제 지정도 없다
    let mut classic = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[65_535]));
    assert_eq!(classic.practice_hint(), None);
    classic.set_forced_flag(Some(Flag::Small(Role::Star)));
    let (outcome, _) = play_aimed(&mut classic, aim_all(Symbol::Star));
    assert_eq!(outcome.flag, Flag::Blank);
    assert_eq!(outcome.payout, 0);
}

#[test]
fn mode_can_only_change_between_games() {
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[]));
    game.set_mode(PlayMode::FreePlay).unwrap();
    assert_eq!(game.mode(), PlayMode::FreePlay);
    game.press_bet().unwrap();
    game.pull_lever().unwrap();
    assert!(matches!(
        game.set_mode(PlayMode::Classic),
        Err(GameError::WrongPhase { .. })
    ));
}

#[test]
fn game_ids_continue_from_the_given_start() {
    let mut game = fruit_star_game::GameController::new(
        profile(),
        fruit_star_core::ReelTiming::default(),
        PlayMode::Classic,
        50,
        9_999,
        1_001,
        Scripted::rolls(&[]),
    )
    .unwrap();
    let (outcome, _) = play_aimed(&mut game, aim_all(Symbol::Bell));
    assert_eq!(outcome.game_id, 1_001);
    assert_eq!(game.next_game_id(), 1_002);
}

/// 1메달을 건 게임에서 추첨 값 `roll`이 가리키는 구간은 3배로 늘어난다
/// (당첨 확률이 베팅 수에 비례하므로). 원하는 플래그가 나오는 스크립트 값을 구한다.
fn one_medal_roll(flag: Flag) -> u32 {
    let start = roll_for(&profile(), flag);
    // 추첨 범위가 3배이므로 스크립트 값 r은 3r 지점을 가리킨다.
    start / 3 + 1
}

#[test]
fn one_bet_adds_a_medal_at_a_time_up_to_the_maximum() {
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[]));
    assert_eq!(game.bet_amount(), 0);
    assert_eq!(game.lit_lines(), 0);
    assert!(game.can_add_bet());

    for (expected_bet, expected_lines) in [(1, 1), (2, 3), (3, 5)] {
        game.press_bet_one().unwrap();
        assert_eq!(game.bet_amount(), expected_bet);
        assert_eq!(game.lit_lines(), expected_lines);
        assert_eq!(game.credit(), 50 - expected_bet);
    }
    assert!(!game.can_add_bet());
    assert_eq!(game.press_bet_one(), Err(GameError::AlreadyBet));
    assert_eq!(game.press_bet(), Err(GameError::AlreadyBet));
    assert_eq!(game.credit(), 47);

    let totals: Vec<u32> = game
        .drain_events()
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::BetPressed {
                amount: 1,
                total,
                replay: false,
            } => Some(total),
            _ => None,
        })
        .collect();
    assert_eq!(totals, vec![1, 2, 3]);
}

#[test]
fn meter_keeps_the_stake_through_the_spin_and_payout() {
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[65_535]));
    game.press_bet_one().unwrap();
    assert_eq!(game.displayed_bet(), 1);
    game.pull_lever().unwrap();
    assert_eq!(game.bet_amount(), 0);
    assert_eq!(game.displayed_bet(), 1);
    spin_up(&mut game);
    for reel in 0..3 {
        game.press_stop(reel).unwrap();
    }
    for _ in 0..100 {
        game.tick(0.02);
        if game.phase() == Phase::Result {
            break;
        }
    }
    assert_eq!(game.phase(), Phase::Result);
    assert_eq!(game.displayed_bet(), 1);
    finish(&mut game);
    assert_eq!(game.displayed_bet(), 0);
}

#[test]
fn max_bet_tops_up_a_partial_bet() {
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[65_535]));
    game.press_bet_one().unwrap();
    game.press_bet().unwrap();
    assert_eq!(game.bet_amount(), 3);
    assert_eq!(game.credit(), 47);
    let (outcome, _) = play_aimed(&mut game, aim_all(Symbol::Bell));
    assert_eq!(outcome.bet, 3);
    assert_eq!(outcome.lines, 5);
    // 1메달 + 2메달 두 건의 원장 항목
    assert_eq!(outcome.bet_entries.len(), 2);
    assert_eq!(outcome.bet_entries.iter().map(|e| e.amount).sum::<u32>(), 3);
}

#[test]
fn a_one_medal_game_uses_only_the_centre_line() {
    let grape = one_medal_roll(Flag::Small(Role::Grape));
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[grape]));
    game.press_bet_one().unwrap();
    let (outcome, _) = play_aimed(&mut game, aim_all(Symbol::Bell));
    assert_eq!(outcome.flag, Flag::Small(Role::Grape));
    assert_eq!(outcome.bet, 1);
    assert_eq!(outcome.lines, 1);
    assert_eq!(outcome.bet_entries.len(), 1);
    // 가운데 줄에서만 성립한다
    assert_eq!(outcome.wins.len(), 1);
    assert_eq!(outcome.wins[0].line, 0);
    assert_eq!(outcome.payout, 8);
    assert_eq!(game.credit(), 50 - 1 + 8);
    // 가운데 줄에 포도가 실제로 늘어서 있다
    for window in &outcome.windows {
        assert_eq!(window[1], Symbol::Grape);
    }
}

#[test]
fn symbols_lined_up_on_an_inactive_line_do_not_pay() {
    // 1메달 게임의 꽝: 위·아래·대각선에 무엇이 늘어서도 당첨이 아니다.
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[65_535]));
    game.press_bet_one().unwrap();
    let (outcome, _) = play_aimed(&mut game, aim_all(Symbol::Grape));
    assert_eq!(outcome.flag, Flag::Blank);
    assert!(outcome.wins.is_empty());
    assert_eq!(outcome.payout, 0);
    // 가운데 줄에는 같은 심볼 세 개가 오지 않는다
    let middle = [
        outcome.windows[0][1],
        outcome.windows[1][1],
        outcome.windows[2][1],
    ];
    assert!(!(middle[0] == middle[1] && middle[1] == middle[2]));
}

#[test]
fn replay_repeats_the_same_bet_size() {
    let replay = one_medal_roll(Flag::Small(Role::Replay));
    let mut game = controller_with(PlayMode::Classic, 50, Scripted::rolls(&[replay, 65_535]));
    game.press_bet_one().unwrap();
    game.press_bet_one().unwrap();
    // 2메달 게임은 추첨 범위가 1.5배다: 스크립트 값 r은 1.5r 지점을 가리키며 리플레이 구간 안이다.
    let (outcome, _) = play_aimed(&mut game, aim_all(Symbol::Star));
    assert!(outcome.replay_awarded);
    assert_eq!(outcome.bet, 2);
    assert_eq!(outcome.lines, 3);
    // 리플레이는 같은 2메달 베팅으로 무료 진행된다
    assert_eq!(game.bet_amount(), 2);
    assert_eq!(game.credit(), 48);
    let (next, _) = play_aimed(&mut game, aim_all(Symbol::Star));
    assert!(next.replay_game);
    assert_eq!(next.bet, 2);
    assert!(next.bet_entries.is_empty());
    assert_eq!(game.credit(), 48);
}

#[test]
fn win_probability_scales_with_the_bet() {
    use fruit_star_core::SplitMix64;
    let table = &profile().lottery;
    let weight = f64::from(table.weight(Flag::Small(Role::Grape)));
    let draws = 200_000;
    for bet in 1..=3u32 {
        let mut rng = SplitMix64::new(u64::from(bet) * 77);
        let hits = (0..draws)
            .filter(|_| table.draw_for_bet(&mut rng, bet, 3) == Flag::Small(Role::Grape))
            .count();
        let expected = weight / 65_536.0 * f64::from(bet) / 3.0;
        let observed = hits as f64 / f64::from(draws);
        assert!(
            (observed - expected).abs() < expected * 0.05,
            "베팅 {bet}: 기대 {expected:.4}, 관측 {observed:.4}"
        );
    }
}
