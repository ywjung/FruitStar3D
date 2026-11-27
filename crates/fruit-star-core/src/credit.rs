//! 가상 메달 원장.
//!
//! 메달 잔액은 이 모듈을 통해서만 변경된다. 모든 변경은 [`LedgerEntry`]로
//! 기록되어 저장 계층이 그대로 영속화할 수 있다. 실제 화폐 가치는 없다.

use crate::error::CreditError;

/// 원장 항목 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LedgerEntryKind {
    /// 가상 메달 투입.
    Insert,
    /// 게임 시작 시 베팅 차감.
    Bet,
    /// 게임 결과에 따른 지급.
    Payout,
}

impl LedgerEntryKind {
    pub const fn key(self) -> &'static str {
        match self {
            LedgerEntryKind::Insert => "insert",
            LedgerEntryKind::Bet => "bet",
            LedgerEntryKind::Payout => "payout",
        }
    }
}

/// 원장 변경 한 건.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerEntry {
    /// 원장 내 일련번호 (1부터 증가).
    pub seq: u64,
    /// 관련 게임 ID. 투입은 게임과 무관하므로 `None`.
    pub game_id: Option<u64>,
    pub kind: LedgerEntryKind,
    pub amount: u32,
    pub balance_after: u32,
}

/// 가상 메달 원장.
#[derive(Debug, Clone)]
pub struct CreditLedger {
    balance: u32,
    insert_limit: u32,
    seq: u64,
    last_settled_game: Option<u64>,
    total_inserted: u64,
    total_bet: u64,
    total_paid: u64,
}

impl CreditLedger {
    /// `insert_limit`은 메달 투입으로 도달할 수 있는 최대 잔액이다.
    pub fn new(initial_balance: u32, insert_limit: u32) -> Self {
        Self {
            balance: initial_balance,
            insert_limit: insert_limit.max(initial_balance),
            seq: 0,
            last_settled_game: None,
            total_inserted: 0,
            total_bet: 0,
            total_paid: 0,
        }
    }

    pub fn balance(&self) -> u32 {
        self.balance
    }

    pub fn insert_limit(&self) -> u32 {
        self.insert_limit
    }

    pub fn total_inserted(&self) -> u64 {
        self.total_inserted
    }

    pub fn total_bet(&self) -> u64 {
        self.total_bet
    }

    pub fn total_paid(&self) -> u64 {
        self.total_paid
    }

    pub fn last_settled_game(&self) -> Option<u64> {
        self.last_settled_game
    }

    fn record(&mut self, game_id: Option<u64>, kind: LedgerEntryKind, amount: u32) -> LedgerEntry {
        self.seq += 1;
        LedgerEntry {
            seq: self.seq,
            game_id,
            kind,
            amount,
            balance_after: self.balance,
        }
    }

    /// 가상 메달을 투입한다.
    pub fn insert(&mut self, amount: u32) -> Result<LedgerEntry, CreditError> {
        if amount == 0 {
            return Err(CreditError::ZeroAmount);
        }
        let next = self
            .balance
            .checked_add(amount)
            .ok_or(CreditError::Overflow)?;
        if next > self.insert_limit {
            return Err(CreditError::LimitExceeded {
                limit: self.insert_limit,
            });
        }
        self.balance = next;
        self.total_inserted += u64::from(amount);
        Ok(self.record(None, LedgerEntryKind::Insert, amount))
    }

    /// 베팅 금액을 차감한다. 잔액이 부족하면 아무것도 변경하지 않는다.
    pub fn bet(&mut self, game_id: u64, amount: u32) -> Result<LedgerEntry, CreditError> {
        if amount == 0 {
            return Err(CreditError::ZeroAmount);
        }
        if self.balance < amount {
            return Err(CreditError::Insufficient {
                need: amount,
                have: self.balance,
            });
        }
        self.balance -= amount;
        self.total_bet += u64::from(amount);
        Ok(self.record(Some(game_id), LedgerEntryKind::Bet, amount))
    }

    /// 게임을 정산한다. 같은 게임은 한 번만 정산할 수 있으며 지급액이 0이어도
    /// 정산 완료로 표시된다 (0이면 원장 항목은 만들지 않는다).
    pub fn settle(
        &mut self,
        game_id: u64,
        payout: u32,
    ) -> Result<Option<LedgerEntry>, CreditError> {
        if self.last_settled_game.is_some_and(|last| game_id <= last) {
            return Err(CreditError::AlreadySettled { game_id });
        }
        let next = self
            .balance
            .checked_add(payout)
            .ok_or(CreditError::Overflow)?;
        self.last_settled_game = Some(game_id);
        if payout == 0 {
            return Ok(None);
        }
        self.balance = next;
        self.total_paid += u64::from(payout);
        Ok(Some(self.record(
            Some(game_id),
            LedgerEntryKind::Payout,
            payout,
        )))
    }
}
