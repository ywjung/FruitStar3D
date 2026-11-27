//! FRUIT STAR 3D 게임 규칙.
//!
//! 한 게임의 흐름은 다음과 같다.
//!
//! 1. 베팅 → 레버: 내부 추첨으로 이번 게임에 허용되는 역할([`Flag`])을 정한다.
//! 2. 정지 버튼: 릴 제어([`control`])가 누른 위치에서 최대 `max_slip`칸 안에서
//!    허용된 역할은 끌어들이고 허용되지 않은 역할은 피하는 정지 위치를 고른다.
//! 3. 평가: 최종 정지 위치에서 보이는 심볼만으로 지급을 계산한다
//!    ([`evaluator`]). 따라서 화면에 보이는 결과와 내부 결과는 항상 일치한다.
//!
//! 추첨 확률과 배당은 이 프로젝트가 독자적으로 정의한 값이며 실제 기기와 무관하다.
//! 메달은 가상이며 금전적 가치가 없다.

pub mod bonus;
pub mod control;
pub mod controller;
pub mod error;
pub mod evaluator;
pub mod events;
pub mod lottery;
pub mod profile;
pub mod rng;

pub use bonus::{BonusKind, BonusState};
pub use control::{Allowed, StopDecision, choose_stop, choose_stop_lines};
pub use controller::{GameController, Phase, PlayMode};
pub use error::{GameError, ProfileError};
pub use evaluator::{
    Evaluation, PAYLINES, Win, Windows, active_lines, evaluate, evaluate_lines, windows_at,
};
pub use events::{GameEvent, GameOutcome};
pub use lottery::{Flag, LOTTERY_DENOMINATOR, LotteryTable};
pub use profile::{BonusRules, FlowTiming, MachineProfile, PayTable, Role};
pub use rng::SystemRandom;
