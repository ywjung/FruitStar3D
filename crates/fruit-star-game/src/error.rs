//! 게임 규칙 계층의 오류.

use fruit_star_core::{CreditError, ReelError};
use thiserror::Error;

use crate::controller::Phase;

/// 기종 프로필 검증 오류.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProfileError {
    #[error("릴 {reel}의 심볼 수가 {actual}개입니다 (필요: {expected}개)")]
    StripLength {
        reel: usize,
        expected: usize,
        actual: usize,
    },
    #[error("릴 {reel}에 '{symbol}' 심볼이 없습니다")]
    MissingSymbol { reel: usize, symbol: &'static str },
    #[error("추첨 가중치 합 {sum}이(가) 분모 {denominator}를 넘습니다")]
    LotteryOverflow { sum: u64, denominator: u32 },
    #[error("추첨표에 '{0}' 항목이 중복되었습니다")]
    DuplicateLotteryEntry(String),
    #[error("{field} 값 {value}은(는) 0~100이어야 합니다")]
    PercentOutOfRange { field: &'static str, value: u32 },
    #[error("최대 미끄러짐 {max_slip}은(는) 릴 길이 {len}보다 작아야 합니다")]
    SlipTooLarge { max_slip: usize, len: usize },
    #[error("'{role}' 배당 {payout}이(가) 게임당 최대 지급 {max}를 넘습니다")]
    PayoutTooLarge {
        role: &'static str,
        payout: u32,
        max: u32,
    },
    #[error("{0} 값은 0보다 커야 합니다")]
    MustBePositive(&'static str),
    #[error("알 수 없는 {kind} '{value}'")]
    UnknownKey { kind: &'static str, value: String },
}

/// 게임 조작 오류. 대부분은 "지금은 받을 수 없는 입력"을 뜻하며 무시해도 안전하다.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GameError {
    #[error("{phase:?} 단계에서는 할 수 없는 조작입니다")]
    WrongPhase { phase: Phase },
    #[error("이미 베팅되어 있습니다")]
    AlreadyBet,
    #[error("베팅이 필요합니다")]
    BetRequired,
    #[error("릴 {reel}은(는) 지금 정지시킬 수 없습니다")]
    StopNotReady { reel: usize },
    #[error("릴 번호 {reel}이(가) 올바르지 않습니다")]
    InvalidReel { reel: usize },
    #[error(transparent)]
    Credit(#[from] CreditError),
    #[error(transparent)]
    Reel(#[from] ReelError),
    #[error(transparent)]
    Profile(#[from] ProfileError),
}
