//! 애플리케이션 전체에서 공유하는 게임 상태.

/// 게임기 전체 상태. 화면 전환과 플레이 진행 단계를 모두 포함한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Loading,
    MainMenu,
    MachineSelect,
    Ready,
    Spinning,
    Stopping,
    Evaluating,
    Result,
    Bonus,
    Paused,
    Error,
}

impl GameState {
    /// 한 게임이 진행 중인 상태인지 여부 (레버 입력 이후 ~ 정산 완료 전).
    pub const fn is_game_in_progress(self) -> bool {
        matches!(
            self,
            GameState::Spinning | GameState::Stopping | GameState::Evaluating | GameState::Result
        )
    }

    /// 게임기 조작 입력을 받는 상태인지 여부.
    pub const fn accepts_machine_input(self) -> bool {
        matches!(
            self,
            GameState::Ready
                | GameState::Spinning
                | GameState::Stopping
                | GameState::Result
                | GameState::Bonus
        )
    }
}
