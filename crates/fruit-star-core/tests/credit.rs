#![allow(clippy::unwrap_used, clippy::expect_used)]

use fruit_star_core::{CreditError, CreditLedger, LedgerEntryKind, RandomSource, SplitMix64};

#[test]
fn insert_bet_and_settle_update_balance_and_totals() {
    let mut ledger = CreditLedger::new(0, 1000);
    let insert = ledger.insert(50).unwrap();
    assert_eq!(insert.kind, LedgerEntryKind::Insert);
    assert_eq!(insert.balance_after, 50);
    assert_eq!(insert.game_id, None);

    let bet = ledger.bet(1, 3).unwrap();
    assert_eq!(bet.kind, LedgerEntryKind::Bet);
    assert_eq!(bet.balance_after, 47);
    assert_eq!(bet.game_id, Some(1));

    let payout = ledger.settle(1, 8).unwrap().unwrap();
    assert_eq!(payout.kind, LedgerEntryKind::Payout);
    assert_eq!(payout.balance_after, 55);
    assert_eq!(payout.seq, 3);

    assert_eq!(ledger.balance(), 55);
    assert_eq!(ledger.total_inserted(), 50);
    assert_eq!(ledger.total_bet(), 3);
    assert_eq!(ledger.total_paid(), 8);
}

#[test]
fn bet_never_drives_balance_negative() {
    let mut ledger = CreditLedger::new(2, 1000);
    assert_eq!(
        ledger.bet(1, 3),
        Err(CreditError::Insufficient { need: 3, have: 2 })
    );
    assert_eq!(ledger.balance(), 2);
    assert_eq!(ledger.total_bet(), 0);
}

#[test]
fn a_game_can_be_settled_only_once() {
    let mut ledger = CreditLedger::new(10, 1000);
    ledger.bet(1, 3).unwrap();
    ledger.settle(1, 15).unwrap();
    assert_eq!(
        ledger.settle(1, 15),
        Err(CreditError::AlreadySettled { game_id: 1 })
    );
    assert_eq!(ledger.balance(), 22);
    // 지급이 0인 게임도 정산 완료로 표시되어 다시 정산할 수 없다
    ledger.bet(2, 3).unwrap();
    assert_eq!(ledger.settle(2, 0).unwrap(), None);
    assert!(ledger.settle(2, 5).is_err());
    // 과거 게임 번호도 거부한다
    assert!(ledger.settle(1, 5).is_err());
    assert_eq!(ledger.balance(), 19);
}

#[test]
fn insert_respects_limit_and_rejects_zero() {
    let mut ledger = CreditLedger::new(0, 100);
    assert_eq!(ledger.insert(0), Err(CreditError::ZeroAmount));
    ledger.insert(100).unwrap();
    assert_eq!(
        ledger.insert(1),
        Err(CreditError::LimitExceeded { limit: 100 })
    );
    assert_eq!(ledger.balance(), 100);
    // 지급은 투입 한도와 무관하게 반영된다
    ledger.bet(1, 3).unwrap();
    ledger.settle(1, 15).unwrap();
    assert_eq!(ledger.balance(), 112);
}

#[test]
fn overflow_is_reported_without_changing_state() {
    let mut ledger = CreditLedger::new(u32::MAX - 1, u32::MAX);
    assert_eq!(ledger.settle(1, 5), Err(CreditError::Overflow));
    assert_eq!(ledger.balance(), u32::MAX - 1);
    assert_eq!(ledger.last_settled_game(), None);
}

#[test]
fn random_below_stays_in_range_and_covers_it() {
    let mut rng = SplitMix64::new(42);
    let mut seen = [false; 7];
    for _ in 0..10_000 {
        let value = rng.below(7);
        assert!(value < 7);
        seen[value as usize] = true;
    }
    assert!(seen.iter().all(|&s| s));
    assert_eq!(rng.below(0), 0);
    assert_eq!(rng.below(1), 0);
}
