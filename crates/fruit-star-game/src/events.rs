//! 게임 진행 중 발생하는 이벤트. 그래픽·음향·저장 계층이 구독한다.

use fruit_star_core::{LedgerEntry, REEL_COUNT};

use crate::bonus::BonusKind;
use crate::controller::PlayMode;
use crate::evaluator::{Win, Windows};
use crate::lottery::Flag;

/// 완료된 한 게임의 전체 기록.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameOutcome {
    pub game_id: u64,
    pub mode: PlayMode,
    /// 이 게임의 베팅 메달 수 (리플레이 게임도 원래 베팅 수를 기록한다).
    pub bet: u32,
    /// 이 게임에서 유효했던 라인 수 (베팅 수에 따라 1, 3, 5).
    pub lines: usize,
    /// 리플레이로 무료 진행된 게임인지 여부.
    pub replay_game: bool,
    /// 보너스 게임 중이었는지 여부.
    pub in_bonus: bool,
    /// 내부 추첨 결과.
    pub flag: Flag,
    /// 릴별 최종 정지 인덱스.
    pub stops: [usize; REEL_COUNT],
    /// 릴별 미끄러진 칸 수.
    pub slips: [usize; REEL_COUNT],
    /// 정지 버튼을 누른 순서 (릴 번호).
    pub stop_order: [usize; REEL_COUNT],
    /// 표시창에 보인 심볼.
    pub windows: Windows,
    pub wins: Vec<Win>,
    /// 지급 메달 수.
    pub payout: u32,
    /// 리플레이가 성립했는지 여부.
    pub replay_awarded: bool,
    /// 이 게임으로 시작된 보너스.
    pub bonus_started: Option<BonusKind>,
    /// 정산 후 잔액.
    pub balance_after: u32,
    /// 이 게임의 베팅 원장 항목. 1메달씩 걸면 여러 건이고, 리플레이 게임이면 비어 있다.
    pub bet_entries: Vec<LedgerEntry>,
    /// 이 게임의 지급 원장 항목 (지급이 없으면 없음).
    pub payout_entry: Option<LedgerEntry>,
}

/// 게임기 이벤트.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    PowerOn,
    PowerOff,
    /// 가상 메달이 투입되었다.
    MedalInserted {
        entry: LedgerEntry,
    },
    /// 베팅되었다. `amount`는 이번에 더한 메달 수, `total`은 현재 걸린 메달 수다.
    /// `replay`가 참이면 리플레이에 의한 자동 베팅이다.
    BetPressed {
        amount: u32,
        total: u32,
        replay: bool,
    },
    LeverPulled {
        game_id: u64,
    },
    ReelStarted(usize),
    StopPressed(usize),
    /// 릴이 정지 위치에 고정되었다.
    ReelStopped {
        reel: usize,
        index: usize,
    },
    GameCompleted(Box<GameOutcome>),
    BonusStarted(BonusKind),
    BonusCompleted {
        kind: BonusKind,
        paid: u32,
        games: u32,
    },
    /// 메달이 배출되었다.
    MedalDispensed(u32),
    /// STAR CHANCE 램프 점등 상태가 바뀌었다.
    ChanceLampChanged(bool),
}
