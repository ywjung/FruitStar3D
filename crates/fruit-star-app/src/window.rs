//! 창, 전체 화면, 프레임 제한, 일시정지·음소거 명령.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::window::{
    Monitor, MonitorSelection, PresentMode, PrimaryMonitor, PrimaryWindow, WindowMode,
    WindowResolution,
};
use fruit_star_config::ConfigSet;
use fruit_star_machine::{AppCommand, AppState, MachineSet, Paused, Settings};

use crate::device::{self, Thermal};

/// 기기 열 상태를 다시 읽는 간격.
const THERMAL_EVERY: Duration = Duration::from_secs(2);

/// 설정에 따른 주 창.
pub fn primary_window(config: &ConfigSet) -> Window {
    let window = &config.application.window;
    Window {
        title: window.title.clone(),
        resolution: WindowResolution::new(window.width, window.height),
        present_mode: if window.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        },
        // 휴대폰에서는 화면 전체를 쓰고 크기를 바꿀 수 없다. 창 크기 설정은 무시된다.
        #[cfg(any(target_os = "ios", target_os = "android"))]
        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
        #[cfg(any(target_os = "ios", target_os = "android"))]
        resizable: false,
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        mode: if window.fullscreen {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            WindowMode::Windowed
        },
        // 트랙패드 핀치로 확대할 수 있게 한다.
        recognize_pinch_gesture: true,
        ..default()
    }
}

pub struct WindowControlPlugin;

impl Plugin for WindowControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameLimiter>()
            .add_systems(
                Update,
                (handle_app_commands, apply_pause)
                    .chain()
                    .after(MachineSet::Input)
                    .before(MachineSet::Simulate),
            )
            .add_systems(Last, limit_frame_rate);
    }
}

fn handle_app_commands(
    mut commands: MessageReader<AppCommand>,
    state: Res<State<AppState>>,
    mut paused: ResMut<Paused>,
    mut settings: ResMut<Settings>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    for command in commands.read() {
        match command {
            AppCommand::ToggleFullscreen => {
                window.mode = match window.mode {
                    WindowMode::Windowed => {
                        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
                    }
                    _ => WindowMode::Windowed,
                };
            }
            AppCommand::ToggleMute => {
                let muted = settings.config.audio.muted;
                settings.config.audio.muted = !muted;
            }
            AppCommand::TogglePause | AppCommand::OpenMenu => {
                if *state.get() == AppState::InGame {
                    paused.0 = !paused.0;
                }
            }
            AppCommand::CycleCamera | AppCommand::ToggleDebug => {}
        }
    }
}

/// 일시정지 중에는 가상 시간을 멈춰 애니메이션과 물리가 함께 선다.
fn apply_pause(paused: Res<Paused>, mut time: ResMut<Time<Virtual>>) {
    if !paused.is_changed() {
        return;
    }
    if paused.0 {
        time.pause();
    } else {
        time.unpause();
    }
}

/// 프레임 제한 상태.
#[derive(Resource, Debug)]
pub struct FrameLimiter {
    last: Instant,
    /// 마지막으로 읽은 기기 열 상태(iOS만)와 읽은 시각.
    thermal: Option<Thermal>,
    thermal_at: Option<Instant>,
}

impl Default for FrameLimiter {
    fn default() -> Self {
        Self {
            last: Instant::now(),
            thermal: None,
            thermal_at: None,
        }
    }
}

/// 이번 프레임의 목표 프레임률. 기기 열 상태가 "높음" 이상이면 `hot_target`과 평소 목표 중
/// 낮은 쪽을 쓴다. 열 상태를 모르면(iOS가 아닌 플랫폼) 평소 목표 그대로다.
pub(crate) fn effective_target(target: u32, hot_target: u32, thermal: Option<Thermal>) -> u32 {
    let target = target.max(1);
    if thermal.is_some_and(Thermal::is_hot) {
        target.min(hot_target.max(1))
    } else {
        target
    }
}

/// 2초마다 기기 열 상태를 읽고, 목표 프레임률이 바뀌면 한 줄 남긴다.
fn refresh_thermal(limiter: &mut FrameLimiter, target: u32, hot_target: u32) {
    let now = Instant::now();
    if limiter
        .thermal_at
        .is_some_and(|at| now.duration_since(at) < THERMAL_EVERY)
    {
        return;
    }
    limiter.thermal_at = Some(now);
    let Some(thermal) = device::thermal() else {
        return;
    };
    let before = effective_target(target, hot_target, limiter.thermal);
    let after = effective_target(target, hot_target, Some(thermal));
    limiter.thermal = Some(thermal);
    if before != after {
        device::report(&format!(
            "기기 열 상태 {}: 목표 프레임률 {before} → {after} FPS",
            thermal.label()
        ));
    }
}

/// 설정한 목표 프레임률을 넘지 않도록 프레임 끝에서 기다린다.
///
/// 수직 동기화가 켜져 있고 모니터 주사율이 목표 이하이면 수직 동기화가 이미
/// 프레임률을 제한하므로 아무것도 하지 않는다. 기기가 뜨거우면 목표를 낮춘다 (`effective_target`).
fn limit_frame_rate(
    settings: Res<Settings>,
    monitor: Query<&Monitor, With<PrimaryMonitor>>,
    mut limiter: ResMut<FrameLimiter>,
) {
    let render = &settings.config.graphics.render;
    refresh_thermal(&mut limiter, render.target_fps, render.hot_target_fps);
    let target = effective_target(render.target_fps, render.hot_target_fps, limiter.thermal);
    let vsync = settings.config.application.window.vsync;
    let refresh_hz = monitor
        .iter()
        .next()
        .and_then(|m| m.refresh_rate_millihertz)
        .map(|millihertz| millihertz as f32 / 1000.0);
    let vsync_is_enough = vsync && refresh_hz.is_some_and(|hz| hz <= target as f32 + 0.5);
    if !vsync_is_enough {
        let frame = Duration::from_secs_f64(1.0 / f64::from(target));
        let now = Instant::now();
        let (wait, deadline) = next_frame(limiter.last, now, frame);
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
        limiter.last = deadline;
    } else {
        limiter.last = Instant::now();
    }
}

/// 다음 프레임까지 기다릴 시간과 이번 프레임의 기준 시각을 구한다.
///
/// 기준 시각을 "깨어난 시각"이 아니라 "이전 기준 + 한 프레임"으로 누적한다. `sleep`이 조금 늦게
/// 깨어나도 다음 대기가 그만큼 짧아져 평균 프레임 간격이 목표와 같아진다. 한 프레임 넘게 밀리면
/// (로딩 등) 따라잡으려 몰아서 그리지 않도록 기준을 지금으로 다시 맞춘다.
pub(crate) fn next_frame(previous: Instant, now: Instant, frame: Duration) -> (Duration, Instant) {
    let deadline = previous + frame;
    if now >= deadline + frame {
        (Duration::ZERO, now)
    } else if now >= deadline {
        (Duration::ZERO, deadline)
    } else {
        (deadline - now, deadline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = Duration::from_micros(8_333);

    #[test]
    fn hot_device_lowers_the_target_and_cool_device_keeps_it() {
        assert_eq!(effective_target(60, 30, None), 60);
        assert_eq!(effective_target(60, 30, Some(Thermal::Nominal)), 60);
        assert_eq!(effective_target(60, 30, Some(Thermal::Fair)), 60);
        assert_eq!(effective_target(60, 30, Some(Thermal::Serious)), 30);
        assert_eq!(effective_target(60, 30, Some(Thermal::Critical)), 30);
        // 뜨거울 때의 목표가 평소보다 높으면 평소 목표를 넘지 않는다
        assert_eq!(effective_target(60, 90, Some(Thermal::Serious)), 60);
        // 0은 1로 본다 (나눗셈 보호)
        assert_eq!(effective_target(0, 0, Some(Thermal::Serious)), 1);
    }

    #[test]
    fn waits_until_the_next_deadline() {
        let start = Instant::now();
        let (wait, deadline) = next_frame(start, start + Duration::from_millis(3), FRAME);
        assert_eq!(deadline, start + FRAME);
        assert_eq!(wait, FRAME - Duration::from_millis(3));
    }

    #[test]
    fn oversleeping_is_paid_back_on_the_next_frame() {
        let start = Instant::now();
        // 첫 프레임 기준은 start + FRAME. 0.5 ms 늦게 깨어 다음 프레임 작업이 1 ms 걸렸다.
        let (_, first) = next_frame(start, start + Duration::from_millis(2), FRAME);
        let late_wake = first + Duration::from_micros(500);
        let (wait, second) = next_frame(first, late_wake + Duration::from_millis(1), FRAME);
        assert_eq!(second, start + FRAME * 2);
        // 늦게 깬 0.5 ms만큼 덜 기다린다
        assert_eq!(wait, FRAME - Duration::from_micros(1_500));
        // 100프레임 동안 기준 시각은 정확히 100 × FRAME만큼 나아간다
        let mut deadline = start;
        for _ in 0..100 {
            let (wait, next) = next_frame(deadline, deadline + Duration::from_millis(1), FRAME);
            assert!(wait <= FRAME);
            deadline = next;
        }
        assert_eq!(deadline, start + FRAME * 100);
    }

    #[test]
    fn a_long_stall_resets_instead_of_catching_up() {
        let start = Instant::now();
        let stalled = start + Duration::from_millis(500);
        let (wait, deadline) = next_frame(start, stalled, FRAME);
        assert!(wait.is_zero());
        assert_eq!(deadline, stalled);
        // 조금(한 프레임 이내) 늦은 경우는 기다리지 않되 기준은 유지한다
        let (wait, deadline) = next_frame(start, start + FRAME + Duration::from_millis(2), FRAME);
        assert!(wait.is_zero());
        assert_eq!(deadline, start + FRAME);
    }
}
