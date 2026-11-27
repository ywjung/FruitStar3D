//! 애플리케이션 상태와 공유 리소스.

use std::path::PathBuf;

use bevy::prelude::*;
use fruit_star_config::ConfigSet;
use fruit_star_core::GameState;
use fruit_star_game::GameController;

/// 화면 전환 상태.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// 리소스 로딩 중.
    #[default]
    Loading,
    MainMenu,
    /// 게임기 앞 (플레이·내부 관찰·갤러리).
    InGame,
    /// 복구할 수 없는 오류. [`FailureMessage`]에 사유가 있다.
    Failed,
}

/// 일시정지 여부. `AppState::InGame`에서만 의미가 있다.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Paused(pub bool);

/// 게임기를 보는 방식.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewMode {
    /// 정면에서 플레이한다.
    #[default]
    Play,
    /// 외장을 숨기고 내부 구조를 보면서 플레이한다.
    Mechanical,
    /// 조작 없이 게임기를 자유롭게 둘러본다.
    Gallery,
}

impl ViewMode {
    /// 게임기 조작 입력을 받는 모드인지 여부.
    pub const fn playable(self) -> bool {
        !matches!(self, ViewMode::Gallery)
    }
}

/// 디버그 표시 단계 (개발 빌드 전용). 0 = 끔, 1 = 수치 표시, 2 = 수치 + 와이어프레임 + 충돌 영역.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DebugLevel(pub u8);

/// `AppState::Failed`의 사유.
#[derive(Resource, Debug, Clone, Default)]
pub struct FailureMessage(pub String);

/// 화면 밖 렌더링 대상. 있으면 주 카메라가 창 대신 이 이미지에 그린다
/// (창이 보이지 않는 환경에서 화면을 저장해 확인하기 위한 것).
#[derive(Resource, Debug, Clone)]
pub struct OffscreenTarget(pub Handle<Image>);

/// 휴대폰처럼 터치로 조작하는 환경인지 여부. 참이면 화면에 터치 조작판을 띄우고
/// 세로 화면에 맞춘 배치와 안전 영역 여백을 쓴다.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchUi(pub bool);

// 기본값이 대상 운영체제에 따라 다르다 (macOS에서는 거짓이라 자동 생성처럼 보인다).
#[allow(clippy::derivable_impls)]
impl Default for TouchUi {
    fn default() -> Self {
        Self(cfg!(any(target_os = "ios", target_os = "android")))
    }
}

/// 화면 UI가 위·아래를 불투명하게 덮는 비율 (화면 높이에 대한 0 ~ 1). 휴대폰 플레이 화면의
/// 상단 표시줄과 하단 조작판이 여기에 해당한다. 카메라는 게임기가 그 사이 빈 곳에 오도록 화면을
/// 위아래로 옮긴다. 덮는 것이 없으면 둘 다 0이다.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct HudInsets {
    pub top: f32,
    pub bottom: f32,
}

/// 게임기를 비추는 주 카메라.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MainCamera;

/// 게임기 한 대의 논리 상태.
#[derive(Resource, Deref, DerefMut)]
pub struct Machine(pub GameController);

/// 전체 설정과 리소스 루트 경로.
#[derive(Resource, Debug, Clone)]
pub struct Settings {
    pub config: ConfigSet,
    /// `assets/`와 `config/`가 들어 있는 디렉터리.
    pub root: PathBuf,
}

/// 화면 전환과 플레이 단계를 합친 전체 게임 상태 (표시·디버그용).
pub fn overall_state(app: AppState, paused: bool, machine: Option<&Machine>) -> GameState {
    match app {
        AppState::Loading => GameState::Loading,
        AppState::MainMenu => GameState::MainMenu,
        AppState::Failed => GameState::Error,
        AppState::InGame if paused => GameState::Paused,
        AppState::InGame => machine.map_or(GameState::Ready, |m| m.game_state()),
    }
}
