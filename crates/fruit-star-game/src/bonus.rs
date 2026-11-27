//! 보너스 게임 상태.

/// 보너스 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BonusKind {
    /// 빨간 7 또는 파란 7 세 개로 시작하는 큰 보너스.
    Big,
    /// BAR 세 개로 시작하는 작은 보너스.
    Regular,
}

impl BonusKind {
    pub const ALL: [BonusKind; 2] = [BonusKind::Big, BonusKind::Regular];

    pub const fn key(self) -> &'static str {
        match self {
            BonusKind::Big => "big",
            BonusKind::Regular => "regular",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// 진행 중인 보너스.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BonusState {
    pub kind: BonusKind,
    /// 보너스 중 지급된 메달 합계.
    pub paid: u32,
    /// 이 값 이상 지급되면 보너스가 끝난다.
    pub payout_cap: u32,
    /// 보너스 중 진행한 게임 수.
    pub games: u32,
}

impl BonusState {
    pub fn new(kind: BonusKind, payout_cap: u32) -> Self {
        Self {
            kind,
            paid: 0,
            payout_cap,
            games: 0,
        }
    }

    /// 보너스 게임 한 판의 결과를 반영하고, 보너스가 끝났는지 돌려준다.
    pub fn record_game(&mut self, payout: u32) -> bool {
        self.games += 1;
        self.paid = self.paid.saturating_add(payout);
        self.is_finished()
    }

    pub fn is_finished(&self) -> bool {
        self.paid >= self.payout_cap
    }

    /// 종료까지 남은 지급 가능 메달 수.
    pub fn remaining(&self) -> u32 {
        self.payout_cap.saturating_sub(self.paid)
    }
}
