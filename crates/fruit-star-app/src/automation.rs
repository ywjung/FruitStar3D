//! 자동화된 실행 확인: 자동 플레이, 스크린샷 예약, 시간 지정 종료.

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use fruit_star_game::{Phase, PlayMode};
use fruit_star_machine::{
    AppCommand, AppState, Machine, MachineCommand, MachineSet, OffscreenTarget, SessionCommand,
    ViewMode,
};

use crate::cli::Options;
use crate::device::{self, DeviceTally, report};

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
    /// 측정 구간의 기기 열 상태·배터리 (iOS만). 1초마다 읽는다.
    device: DeviceTally,
    device_at: Option<f32>,
    /// 중간 보고를 낼 다음 시각 (초). 실행이 도중에 끊겨도 그때까지의 값이 남게 한다.
    next_report: f32,
    /// `--exit-after`로 종료를 요청한 시각 (초).
    exited_at: Option<f32>,
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
            device: DeviceTally::default(),
            device_at: None,
            next_report: REPORT_EVERY,
            exited_at: None,
        }
    }
}

/// 중간 보고 간격 (초).
const REPORT_EVERY: f32 = 60.0;

impl Automation {
    /// 지금까지의 프레임 시간 요약. 집계한 프레임이 없으면 `None`.
    fn perf_summary(&self) -> Option<String> {
        (self.frames > 0).then(|| {
            let average = self.frame_sum / self.frames as f32;
            format!(
                "{} 프레임, 평균 {:.2} ms ({:.1} FPS), 최대 {:.2} ms",
                self.frames,
                average * 1000.0,
                1.0 / average,
                self.frame_max * 1000.0
            )
        })
    }
}

pub struct AutomationPlugin;

impl Plugin for AutomationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::MainMenu),
            (auto_start, keep_screen_on_for_auto_play),
        )
        .add_systems(Update, auto_play.in_set(MachineSet::Input))
        .add_systems(Update, scheduled_actions);
        #[cfg(target_os = "ios")]
        app.add_systems(
            Last,
            end_process_after_exit.after(crate::persistence::close_on_exit),
        );
    }
}

/// iOS에서는 winit이 종료 요청을 무시하고(`ControlFlow::Exit ignored on iOS`), 종료를 요청한 뒤로는
/// Bevy도 프레임을 더 돌리지 않는다. 그대로 두면 앱이 화면을 켠 채 남으므로, 자동화가 종료를 요청한
/// 그 프레임의 마지막(열린 세션을 닫은 뒤)에 프로세스를 끝낸다. `--exit-after`로만 오는 경로이며,
/// 일반 플레이에는 종료 단추가 없다.
#[cfg(target_os = "ios")]
fn end_process_after_exit(automation: Res<Automation>) {
    if automation.exited_at.is_some() {
        std::process::exit(0);
    }
}

fn auto_start(mut automation: ResMut<Automation>, mut sessions: MessageWriter<SessionCommand>) {
    if let Some((mode, view)) = automation.start.take() {
        sessions.write(SessionCommand::Start { mode, view });
    }
}

/// 자동 플레이는 화면을 만지지 않으므로, 측정 중 화면이 꺼져 앱이 멈추지 않게 자동 잠금을 끈다.
/// 일반 플레이의 화면 잠금 동작은 바꾸지 않는다. 메인 스레드에서 돌도록 `NonSendMarker`를 받는다.
fn keep_screen_on_for_auto_play(_main_thread: NonSendMarker, automation: Res<Automation>) {
    if automation.auto_play {
        device::keep_screen_on(true);
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

/// 기기 상태를 읽는 iOS API는 메인 스레드에서만 부를 수 있어 `NonSendMarker`를 받는다.
fn scheduled_actions(
    _main_thread: NonSendMarker,
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
    if automation.exited_at.is_some() {
        return;
    }
    if delta > 0.25 {
        let line = format!("긴 프레임: {:.0} ms (시작 후 {:.2}초)", delta * 1000.0, now);
        warn!("{line}");
        #[cfg(target_os = "ios")]
        eprintln!("{line}");
    }
    if now > 2.0 {
        automation.frames += 1;
        automation.frame_sum += delta;
        automation.frame_max = automation.frame_max.max(delta);
        let since = automation.device_at.map_or(now - 2.0, |at| now - at);
        if automation.device_at.is_none() || since >= 1.0 {
            if let Some(sample) = device::read() {
                automation.device.record(sample, since);
            }
            automation.device_at = Some(now);
        }
        if now >= automation.next_report {
            automation.next_report += REPORT_EVERY;
            if let Some(perf) = automation.perf_summary() {
                report(&format!("중간 측정 ({now:.0}초): {perf}"));
            }
            if let Some(device) = automation.device.summary() {
                report(&format!("중간 측정 ({now:.0}초): {device}"));
            }
        }
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
        automation.exited_at = Some(now);
        if let Some(perf) = automation.perf_summary() {
            report(&format!("성능 측정: {perf}"));
        }
        if let Some(device) = automation.device.summary() {
            report(&device);
        }
        exit.write(AppExit::Success);
    }
}
