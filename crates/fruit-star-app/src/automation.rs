//! 자동화된 실행 확인: 자동 플레이, 스크린샷 예약, 시간 지정 종료.

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use fruit_star_game::{Phase, PlayMode};
use fruit_star_machine::{
    AppCommand, AppState, Machine, MachineCommand, MachineSet, OffscreenTarget, SessionCommand,
    ViewMode,
};

use crate::cli::Options;

/// 명령줄 옵션에서 온 자동화 설정.
#[derive(Resource, Debug, Clone, Default)]
pub struct Automation {
    start: Option<(PlayMode, ViewMode)>,
    auto_play: bool,
    screenshots: Vec<(f32, PathBuf)>,
    camera_at: Vec<f32>,
    pause_at: Vec<f32>,
    exit_after: Option<f32>,
    /// 게임기 앞에 앉은 뒤 흐른 시간 (초). 메뉴만 띄운 경우에는 메뉴 진입 뒤 시간.
    elapsed: f32,
    wait: f32,
    /// 프레임 시간 통계 (초): 프레임 수, 합계, 최댓값. 첫 2초는 준비 시간으로 보고 제외한다.
    frames: u32,
    frame_sum: f32,
    frame_max: f32,
}

impl Automation {
    pub fn from_options(options: &Options) -> Self {
        Self {
            start: options
                .mode
                .map(|mode| (mode, options.view.unwrap_or_default())),
            auto_play: options.auto_play,
            screenshots: options.screenshots.clone(),
            camera_at: options.camera_at.clone(),
            pause_at: options.pause_at.clone(),
            exit_after: options.exit_after,
            elapsed: 0.0,
            wait: 0.0,
            frames: 0,
            frame_sum: 0.0,
            frame_max: 0.0,
        }
    }
}

pub struct AutomationPlugin;

impl Plugin for AutomationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), auto_start)
            .add_systems(Update, auto_play.in_set(MachineSet::Input))
            .add_systems(Update, scheduled_actions);
    }
}

fn auto_start(mut automation: ResMut<Automation>, mut sessions: MessageWriter<SessionCommand>) {
    if let Some((mode, view)) = automation.start.take() {
        sessions.write(SessionCommand::Start { mode, view });
    }
}

/// 사람처럼 일정한 간격으로 베팅·레버·정지를 누른다.
fn auto_play(
    time: Res<Time>,
    state: Res<State<AppState>>,
    machine: Res<Machine>,
    mut automation: ResMut<Automation>,
    mut commands: MessageWriter<MachineCommand>,
) {
    if !automation.auto_play || *state.get() != AppState::InGame {
        return;
    }
    automation.wait -= time.delta_secs();
    if automation.wait > 0.0 {
        return;
    }
    match machine.phase() {
        Phase::Ready => {
            if machine.bet_placed() {
                commands.write(MachineCommand::Lever);
                automation.wait = 1.1;
            } else if machine.credit() < machine.profile().bet && !machine.mode().auto_refill() {
                commands.write(MachineCommand::InsertMedal);
                automation.wait = 0.6;
            } else {
                commands.write(MachineCommand::Bet);
                automation.wait = 0.35;
            }
        }
        Phase::Spinning | Phase::Stopping => {
            if let Some(reel) = (0..3).find(|&reel| machine.can_stop(reel)) {
                commands.write(MachineCommand::Stop(reel));
                automation.wait = 0.55;
            }
        }
        Phase::Evaluating | Phase::Result => {}
    }
}

fn scheduled_actions(
    mut commands: Commands,
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    mut automation: ResMut<Automation>,
    offscreen: Option<Res<OffscreenTarget>>,
    mut app_commands: MessageWriter<AppCommand>,
    mut exit: MessageWriter<AppExit>,
) {
    // 로딩 시간은 기기마다 다르므로 로딩이 끝난 뒤부터 잰다.
    if *state.get() == AppState::Loading || automation.start.is_some() {
        return;
    }
    let delta = time.delta_secs();
    automation.elapsed += delta;
    let now = automation.elapsed;
    if delta > 0.25 {
        warn!("긴 프레임: {:.0} ms (시작 후 {:.2}초)", delta * 1000.0, now);
    }
    if now > 2.0 {
        automation.frames += 1;
        automation.frame_sum += delta;
        automation.frame_max = automation.frame_max.max(delta);
    }

    let (due, pending): (Vec<_>, Vec<_>) = automation
        .screenshots
        .drain(..)
        .partition(|(at, _)| *at <= now);
    automation.screenshots = pending;
    for (_, path) in due {
        info!("스크린샷 저장: {}", path.display());
        let source = match &offscreen {
            Some(target) => Screenshot::image(target.0.clone()),
            None => Screenshot::primary_window(),
        };
        commands.spawn(source).observe(save_to_disk(path));
    }

    let before = automation.camera_at.len();
    automation.camera_at.retain(|at| *at > now);
    for _ in automation.camera_at.len()..before {
        app_commands.write(AppCommand::CycleCamera);
    }
    let before = automation.pause_at.len();
    automation.pause_at.retain(|at| *at > now);
    for _ in automation.pause_at.len()..before {
        app_commands.write(AppCommand::TogglePause);
    }

    if automation.exit_after.is_some_and(|limit| now >= limit) {
        automation.exit_after = None;
        if automation.frames > 0 {
            let average = automation.frame_sum / automation.frames as f32;
            let summary = format!(
                "성능 측정: {} 프레임, 평균 {:.2} ms ({:.1} FPS), 최대 {:.2} ms",
                automation.frames,
                average * 1000.0,
                1.0 / average,
                automation.frame_max * 1000.0
            );
            info!("{summary}");
            // iOS의 로그는 시스템 로그로만 가므로, `devicectl --console`로 받을 수 있게 표준 오류에도 쓴다.
            #[cfg(target_os = "ios")]
            eprintln!("{summary}");
        }
        exit.write(AppExit::Success);
    }
}
