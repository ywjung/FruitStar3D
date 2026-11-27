//! 게임기 ECS 계층.
//!
//! 순수 도메인([`fruit_star_game::GameController`])을 Bevy 리소스로 감싸고,
//! 입력 명령을 도메인에 전달하며, 도메인 이벤트를 Bevy 메시지로 내보낸다.
//! 레버·버튼·표시기·램프의 *논리 상태*(눌림 정도, 점등 여부, 밝기)를 계산하고,
//! 실제 메시와 재질에 반영하는 일은 렌더링 계층이 맡는다.

pub mod button;
pub mod display;
pub mod lamp;
pub mod layout;
pub mod medal;
pub mod messages;
pub mod plugin;
pub mod state;

pub use button::{PartKind, Pressable};
pub use display::{DataCounter, DisplayKind, DisplayValues, SevenSegment};
pub use lamp::{Lamp, LampKind, LampMode, LedStrip, StatusLamp};
pub use medal::{Medal, PayoutCounter};
pub use messages::{
    AppCommand, CameraControl, Clickable, MachineCommand, MachineEvent, MedalCollision,
    MedalPaidOut, Notice, PartPressed, SessionCommand, StatsSnapshot, UiClick,
};
pub use plugin::{MachinePlugin, MachineSet};
pub use state::{
    AppState, DebugLevel, FailureMessage, Machine, MainCamera, OffscreenTarget, Paused, Settings,
    TouchUi, ViewMode, overall_state,
};
