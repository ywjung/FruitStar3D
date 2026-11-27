#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use fruit_star_core::{ReelTiming, SplitMix64};
use fruit_star_game::{GameController, GameEvent, GameOutcome, MachineProfile, Phase, PlayMode};
use fruit_star_storage::{Achievement, Statistics, Store, StoreError};
use rusqlite::Connection;

fn temp_db(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fruit-star-store-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("nested").join("fruitstar.sqlite3")
}

fn controller(mode: PlayMode, credit: u32, first_game: u64, seed: u64) -> GameController {
    GameController::new(
        MachineProfile::fruit_classic(),
        ReelTiming::default(),
        mode,
        credit,
        9_999,
        first_game,
        Box::new(SplitMix64::new(seed)),
    )
    .unwrap()
}

/// 한 게임을 끝까지 진행하고 결과를 돌려준다.
fn play(game: &mut GameController) -> GameOutcome {
    if !game.bet_placed() {
        game.press_bet().unwrap();
    }
    game.pull_lever().unwrap();
    game.tick(0.5);
    for reel in 0..3 {
        game.tick(0.037);
        game.press_stop(reel).unwrap();
    }
    let mut outcome = None;
    for _ in 0..400 {
        game.tick(0.05);
        for event in game.drain_events() {
            if let GameEvent::GameCompleted(o) = event {
                outcome = Some(*o);
            }
        }
        if game.phase() == Phase::Ready {
            break;
        }
    }
    outcome.unwrap()
}

fn open_with_machine() -> Store {
    let mut store = Store::in_memory().unwrap();
    store
        .upsert_machine(&MachineProfile::fruit_classic())
        .unwrap();
    store
}

#[test]
fn fresh_database_is_migrated_and_empty() {
    let store = open_with_machine();
    assert_eq!(store.schema_version().unwrap(), 1);
    assert_eq!(store.statistics().unwrap(), Statistics::default());
    assert_eq!(store.next_game_id().unwrap(), 1);
    assert_eq!(store.classic_balance().unwrap(), None);
    assert!(store.achievements().unwrap().is_empty());
}

#[test]
fn games_and_ledger_entries_are_recorded_together() {
    let mut store = open_with_machine();
    let mut game = controller(PlayMode::Classic, 300, 1, 11);
    let session = store
        .begin_session("fruit_classic", PlayMode::Classic, 300)
        .unwrap();

    let mut expected = Statistics {
        sessions: 1,
        ..Statistics::default()
    };
    for _ in 0..60 {
        let outcome = play(&mut game);
        store.record_game(session, &outcome).unwrap();
        expected.games += 1;
        if !outcome.replay_game {
            expected.total_bet += u64::from(outcome.bet);
        }
        expected.total_payout += u64::from(outcome.payout);
        expected.replays += u64::from(outcome.replay_awarded);
        expected.max_payout = expected.max_payout.max(u64::from(outcome.payout));
        match outcome.bonus_started.map(|k| k.key()) {
            Some("big") => expected.big_bonuses += 1,
            Some(_) => expected.regular_bonuses += 1,
            None => {}
        }
        // 저장된 보유 메달은 항상 원장 잔액과 같다
        assert_eq!(store.classic_balance().unwrap(), Some(game.credit()));
    }
    let stats = store.statistics().unwrap();
    assert_eq!(
        Statistics {
            play_seconds: 0,
            ..stats
        },
        expected
    );
    // 원장 합계와 통계가 일치한다
    assert_eq!(stats.total_bet, game.ledger().total_bet());
    assert_eq!(stats.total_payout, game.ledger().total_paid());
    assert_eq!(store.next_game_id().unwrap(), 61);
    assert!(
        expected.total_payout > 0,
        "60게임이면 포도가 한 번은 나온다"
    );
    assert!(
        store
            .achievements()
            .unwrap()
            .contains(&Achievement::FirstWin)
    );

    store.end_session(session, game.credit()).unwrap();
    // 이미 닫힌 세션을 다시 닫아도 오류가 아니다
    store.end_session(session, game.credit()).unwrap();
    assert!(matches!(
        store.end_session(9_999, 0),
        Err(StoreError::UnknownSession(9_999))
    ));
}

#[test]
fn duplicate_game_id_is_rejected_and_leaves_no_partial_rows() {
    let path = temp_db("duplicate");
    let mut store = Store::open(&path).unwrap();
    store
        .upsert_machine(&MachineProfile::fruit_classic())
        .unwrap();
    let mut game = controller(PlayMode::Classic, 100, 1, 3);
    let session = store
        .begin_session("fruit_classic", PlayMode::Classic, 100)
        .unwrap();
    let outcome = play(&mut game);
    store.record_game(session, &outcome).unwrap();
    let before = store.statistics().unwrap();

    assert!(matches!(
        store.record_game(session, &outcome),
        Err(StoreError::DuplicateGame(1))
    ));
    assert_eq!(store.statistics().unwrap(), before);
    drop(store);

    // 원장 항목도 두 번 들어가지 않았다
    let conn = Connection::open(&path).unwrap();
    let credit_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM credit_history", [], |r| r.get(0))
        .unwrap();
    let expected = outcome.bet_entries.len() as i64 + i64::from(outcome.payout_entry.is_some());
    assert_eq!(credit_rows, expected);
    let reel_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM reel_results", [], |r| r.get(0))
        .unwrap();
    assert_eq!(reel_rows, 3);
    let games: i64 = conn
        .query_row("SELECT games FROM game_sessions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(games, 1);
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn recording_into_an_unknown_session_rolls_back() {
    let mut store = open_with_machine();
    let mut game = controller(PlayMode::Classic, 100, 1, 5);
    let outcome = play(&mut game);
    assert!(store.record_game(42, &outcome).is_err());
    assert_eq!(store.statistics().unwrap().games, 0);
    assert_eq!(store.classic_balance().unwrap(), None);
    assert_eq!(store.next_game_id().unwrap(), 1);
}

#[test]
fn reel_results_match_the_outcome() {
    let path = temp_db("reels");
    let mut store = Store::open(&path).unwrap();
    store
        .upsert_machine(&MachineProfile::fruit_classic())
        .unwrap();
    let mut game = controller(PlayMode::Classic, 100, 1, 21);
    let session = store
        .begin_session("fruit_classic", PlayMode::Classic, 100)
        .unwrap();
    let outcome = play(&mut game);
    store.record_game(session, &outcome).unwrap();
    drop(store);

    let conn = Connection::open(&path).unwrap();
    let mut statement = conn
        .prepare(
            "SELECT reel, stop_index, slip, top, middle, bottom FROM reel_results ORDER BY reel",
        )
        .unwrap();
    let rows: Vec<(i64, i64, i64, String, String, String)> = statement
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows.len(), 3);
    for (reel, stop, slip, top, middle, bottom) in rows {
        let reel = reel as usize;
        assert_eq!(stop as usize, outcome.stops[reel]);
        assert_eq!(slip as usize, outcome.slips[reel]);
        let window = outcome.windows[reel];
        assert_eq!(
            [top.as_str(), middle.as_str(), bottom.as_str()],
            window.map(|s| s.key())
        );
    }
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn data_survives_reopen_and_unclean_shutdown_is_recovered() {
    let path = temp_db("recover");
    let final_balance;
    {
        let mut store = Store::open(&path).unwrap();
        store
            .upsert_machine(&MachineProfile::fruit_classic())
            .unwrap();
        let mut game = controller(PlayMode::Classic, 200, 1, 99);
        let session = store
            .begin_session("fruit_classic", PlayMode::Classic, 200)
            .unwrap();
        game.insert_medals(50).unwrap();
        for event in game.drain_events() {
            if let GameEvent::MedalInserted { entry } = event {
                store
                    .record_credit(session, PlayMode::Classic, &entry)
                    .unwrap();
            }
        }
        for _ in 0..10 {
            let outcome = play(&mut game);
            store.record_game(session, &outcome).unwrap();
        }
        final_balance = game.credit();
        // 베팅만 하고 게임을 끝내지 못한 채 종료 (세션을 닫지 않음) → 그 베팅은 기록되지 않는다
        game.press_bet().unwrap();
    }

    // 보유 메달 값이 손상된 상황을 흉내 낸다
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE user_settings SET value = '123456' WHERE key = 'internal.classic_balance'",
            [],
        )
        .unwrap();
    }

    let mut store = Store::open(&path).unwrap();
    let report = store.recover().unwrap();
    assert_eq!(report.unclean_sessions, 1);
    assert!(report.balance_repaired);
    assert_eq!(store.classic_balance().unwrap(), Some(final_balance));
    assert_eq!(store.statistics().unwrap().games, 10);
    assert_eq!(store.next_game_id().unwrap(), 11);

    // 두 번째 복구는 할 일이 없다
    let report = store.recover().unwrap();
    assert_eq!(report.unclean_sessions, 0);
    assert!(!report.balance_repaired);
    drop(store);

    let conn = Connection::open(&path).unwrap();
    let (clean, end_balance): (i64, i64) = conn
        .query_row(
            "SELECT clean_exit, end_balance FROM game_sessions",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(clean, 0);
    assert_eq!(end_balance as u32, final_balance);
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}

#[test]
fn free_play_does_not_touch_the_classic_balance() {
    let mut store = open_with_machine();
    let mut classic = controller(PlayMode::Classic, 100, 1, 1);
    let session = store
        .begin_session("fruit_classic", PlayMode::Classic, 100)
        .unwrap();
    let outcome = play(&mut classic);
    store.record_game(session, &outcome).unwrap();
    let saved = store.classic_balance().unwrap();

    let mut free = controller(PlayMode::FreePlay, 0, 2, 2);
    let session = store
        .begin_session("fruit_classic", PlayMode::FreePlay, 0)
        .unwrap();
    for _ in 0..5 {
        let outcome = play(&mut free);
        store.record_game(session, &outcome).unwrap();
    }
    for event in free.drain_events() {
        if let GameEvent::MedalInserted { entry } = event {
            store
                .record_credit(session, PlayMode::FreePlay, &entry)
                .unwrap();
        }
    }
    assert_eq!(store.classic_balance().unwrap(), saved);
    assert_eq!(store.recover().unwrap().unclean_sessions, 2);
    assert_eq!(
        store.classic_balance().unwrap(),
        saved,
        "복구도 다른 모드의 잔액을 쓰지 않는다"
    );
}

#[test]
fn settings_round_trip_and_survive_history_reset() {
    let mut store = open_with_machine();
    assert_eq!(store.setting("audio.master").unwrap(), None);
    store.set_setting("audio.master", "0.5").unwrap();
    store.set_setting("audio.master", "0.25").unwrap();
    store.set_setting("game.language", "ja").unwrap();
    assert_eq!(
        store.setting("audio.master").unwrap().as_deref(),
        Some("0.25")
    );

    let mut game = controller(PlayMode::Classic, 100, 1, 8);
    let session = store
        .begin_session("fruit_classic", PlayMode::Classic, 100)
        .unwrap();
    for _ in 0..20 {
        let outcome = play(&mut game);
        store.record_game(session, &outcome).unwrap();
    }
    assert!(store.statistics().unwrap().games > 0);

    store.reset_history().unwrap();
    assert_eq!(store.statistics().unwrap(), Statistics::default());
    assert_eq!(store.classic_balance().unwrap(), None);
    assert!(store.achievements().unwrap().is_empty());
    assert_eq!(store.next_game_id().unwrap(), 1);
    assert_eq!(
        store.setting("game.language").unwrap().as_deref(),
        Some("ja")
    );
}

#[test]
fn achievements_unlock_once() {
    let mut store = open_with_machine();
    let mut game = controller(PlayMode::FreePlay, 0, 1, 4);
    let session = store
        .begin_session("fruit_classic", PlayMode::FreePlay, 0)
        .unwrap();
    let mut unlocked = Vec::new();
    for _ in 0..120 {
        let outcome = play(&mut game);
        unlocked.extend(store.record_game(session, &outcome).unwrap());
    }
    assert!(unlocked.contains(&Achievement::FirstWin));
    assert!(unlocked.contains(&Achievement::Games100));
    let mut unique = unlocked.clone();
    unique.sort_by_key(|a| a.key());
    unique.dedup();
    assert_eq!(
        unique.len(),
        unlocked.len(),
        "같은 달성이 두 번 통지되면 안 된다"
    );
    assert_eq!(store.achievements().unwrap().len(), unlocked.len());
}

#[test]
fn newer_schema_is_refused() {
    let path = temp_db("newer");
    drop(Store::open(&path).unwrap());
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (999, 0)",
            [],
        )
        .unwrap();
    }
    assert!(matches!(
        Store::open(&path),
        Err(StoreError::SchemaTooNew {
            found: 999,
            supported: 1
        })
    ));
    std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
}
