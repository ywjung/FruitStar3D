//! 계층 사이를 오가는 메시지.

use bevy::prelude::*;
use fruit_star_game::{GameEvent, PlayMode};

use crate::button::PartKind;
use crate::state::ViewMode;

/// 게임기 조작 명령. 입력 계층이 보내고 [`crate::plugin`]이 도메인에 전달한다.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MachineCommand {
    /// 가상 메달 추가.
    InsertMedal,
    /// MAX BET (최대 베팅까지 한 번에).
    Bet,
    /// 1 BET (메달 한 장).
    BetOne,
    /// START 레버.
    Lever,
    /// 정지 버튼 (0=왼쪽, 1=가운데, 2=오른쪽).
    Stop(usize),
}

impl MachineCommand {
    /// 이 명령으로 물리적으로 움직이는 부품.
    pub const fn part(self) -> PartKind {
        match self {
            MachineCommand::InsertMedal => PartKind::MedalSlot,
            MachineCommand::Bet => PartKind::Bet,
            MachineCommand::BetOne => PartKind::BetOne,
            MachineCommand::Lever => PartKind::Lever,
            MachineCommand::Stop(reel) => PartKind::Stop(reel),
        }
    }
}

/// 도메인에서 일어난 이벤트. 그래픽·음향·저장 계층이 구독한다.
#[derive(Message, Debug, Clone)]
pub struct MachineEvent(pub GameEvent);

/// 게임기 밖의 조작 (화면·음향·메뉴).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppCommand {
    CycleCamera,
    ToggleFullscreen,
    ToggleMute,
    TogglePause,
    OpenMenu,
    ToggleDebug,
}

/// 부품이 물리적으로 눌렸다. 입력이 도메인에서 거부되었더라도 발생한다
/// (버튼은 눌리지만 아무 일도 일어나지 않는 상황).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartPressed {
    pub part: PartKind,
    /// 도메인이 입력을 받아들였는지 여부.
    pub accepted: bool,
}

/// 메달 한 장이 배출구로 나왔다.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MedalPaidOut {
    /// 이 장을 제외하고 아직 나올 메달 수.
    pub remaining: u32,
}

/// 마우스·트랙패드로 눌러 조작할 수 있는 부품에 붙이는 표시.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clickable(pub MachineCommand);

/// 사용자의 카메라 조작 (드래그·스크롤·핀치).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct CameraControl {
    /// 회전량 (라디안): x = 좌우, y = 상하.
    pub orbit: Vec2,
    /// 확대 비율 변화. 양수면 가까워진다.
    pub zoom: f32,
}

/// 플레이 세션 제어. UI가 보내고 앱이 처리한다.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCommand {
    /// 지정한 모드와 보기 방식으로 게임기 앞에 앉는다.
    Start { mode: PlayMode, view: ViewMode },
    /// 메뉴로 돌아간다.
    End,
    /// 플레이 기록을 모두 지운다.
    ResetHistory,
    /// 프로그램을 끝낸다.
    Quit,
}

/// 사용자에게 잠깐 보여 줄 알림. `key`는 UI가 번역하는 메시지 키다.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub key: String,
}

/// 저장소에서 읽은 누적 통계의 사본 (표시용).
#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct StatsSnapshot {
    pub games: u64,
    pub play_seconds: u64,
    pub total_bet: u64,
    pub total_payout: u64,
    pub replays: u64,
    pub big_bonuses: u64,
    pub regular_bonuses: u64,
    pub max_payout: u64,
    /// 달성한 기록의 키.
    pub achievements: Vec<String>,
    /// 기록을 디스크에 저장하고 있는지 여부 (저장소를 열지 못하면 거짓).
    pub persistent: bool,
}

/// 메달이 받침이나 다른 메달에 부딪혔다 (충돌음용).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct MedalCollision {
    pub position: Vec3,
    /// 다른 메달과의 접촉이면 두 금속 모서리가 짧게 울린다.
    pub other_medal: bool,
    /// 충돌 세기 (0~1).
    pub strength: f32,
}

/// UI에서 항목을 선택했다 (선택음용).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiClick;
