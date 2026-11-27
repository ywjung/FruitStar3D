//! 도메인 오류 타입.

use thiserror::Error;

use crate::reel::ReelState;

/// 릴 조작 오류.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReelError {
    #[error("릴 {reel}: 심볼 배열이 비어 있습니다")]
    EmptyStrip { reel: u8 },
    #[error("릴 {reel}: {state:?} 상태에서는 회전을 시작할 수 없습니다")]
    CannotStart { reel: u8, state: ReelState },
    #[error("릴 {reel}: {state:?} 상태에서는 정지시킬 수 없습니다")]
    CannotStop { reel: u8, state: ReelState },
    #[error("릴 {reel}: 정지 위치 {index}가 심볼 수 {len}을 벗어났습니다")]
    IndexOutOfRange { reel: u8, index: usize, len: usize },
    #[error("릴 타이밍 설정이 올바르지 않습니다: {0}")]
    InvalidTiming(&'static str),
}

/// 가상 메달 원장 오류.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CreditError {
    #[error("메달이 부족합니다 (필요 {need}, 보유 {have})")]
    Insufficient { need: u32, have: u32 },
    #[error("금액은 0보다 커야 합니다")]
    ZeroAmount,
    #[error("크레딧 한도 {limit}를 초과합니다")]
    LimitExceeded { limit: u32 },
    #[error("게임 {game_id}은(는) 이미 정산되었습니다")]
    AlreadySettled { game_id: u64 },
    #[error("크레딧 값이 표현 범위를 넘었습니다")]
    Overflow,
}
