//! FRUIT STAR 3D 핵심 도메인.
//!
//! 이 크레이트는 Bevy나 렌더링에 의존하지 않는 순수 Rust 코드이며,
//! 헤드리스 환경에서 테스트할 수 있다.

pub mod credit;
pub mod error;
pub mod game_state;
pub mod reel;
pub mod rng;
pub mod symbol;

pub use credit::{CreditLedger, LedgerEntry, LedgerEntryKind};
pub use error::{CreditError, ReelError};
pub use game_state::GameState;
pub use reel::{Reel, ReelState, ReelTiming, ReelUpdate, VISIBLE_ROWS};
pub use rng::{RandomSource, SplitMix64};
pub use symbol::Symbol;

/// 릴 개수. 이 게임기는 3릴 기종이다.
pub const REEL_COUNT: usize = 3;
