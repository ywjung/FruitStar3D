//! FRUIT STAR 3D 입력 계층.
//!
//! 키보드, 마우스, 트랙패드 입력을 게임기 명령([`MachineCommand`])과 앱 명령
//! ([`AppCommand`]), 카메라 조작([`CameraControl`])으로 바꾼다. 게임 규칙은 모른다.

pub mod keymap;

use bevy::input::gestures::PinchGesture;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::picking::events::{PointerPress, PointerRelease};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use bevy::window::{AppLifecycle, WindowFocused};
use fruit_star_machine::{
    AppCommand, AppState, CameraControl, Clickable, MachineCommand, MachineSet, Paused, Settings,
};

pub use keymap::{Action, KeyMap, parse_key};

/// 입력 플러그인.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>()
            .init_resource::<PointerCapture>()
            .init_resource::<PauseOnFocusLoss>()
            .add_systems(Startup, build_keymap)
            .add_systems(
                Update,
                (track_focus, keyboard_input, pointer_camera)
                    .chain()
                    .in_set(MachineSet::Input),
            )
            .add_observer(press_part)
            .add_observer(release_pointer);
    }
}

/// 창이 입력 포커스를 가지고 있는지 여부.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputFocus(pub bool);

impl Default for InputFocus {
    fn default() -> Self {
        Self(true)
    }
}

/// 창이 포커스를 잃으면 게임을 일시정지할지 여부. 기본값은 켜짐이다.
///
/// 자동 플레이·화면 밖 렌더링처럼 사람이 창을 보지 않는 실행에서는 끈다. 그런 실행은 처음부터
/// 포커스가 없어서, 켜 두면 플레이를 시작하자마자 멈춘다.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseOnFocusLoss(pub bool);

impl Default for PauseOnFocusLoss {
    fn default() -> Self {
        Self(true)
    }
}

/// 포인터를 누른 채 끌고 있을 때, 그 누름이 게임기 부품에서 시작되었는지 여부.
/// 부품을 누른 채 끄는 동작으로 카메라가 돌지 않게 한다.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PointerCapture(bool);

fn build_keymap(mut commands: Commands, settings: Res<Settings>) {
    let (map, unknown) = KeyMap::from_config(&settings.config.controls.keys);
    for name in unknown {
        warn!("알 수 없는 키 이름 '{name}'은(는) 무시합니다.");
    }
    commands.insert_resource(map);
}

/// 포커스를 잃으면 게임을 일시정지하고 눌린 키 상태를 모두 풀어, 돌아왔을 때 키가 눌린 채로 남거나
/// 의도하지 않은 입력이 들어가는 일을 막는다.
fn track_focus(
    mut events: MessageReader<WindowFocused>,
    mut lifecycle: MessageReader<AppLifecycle>,
    mut focus: ResMut<InputFocus>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut buttons: ResMut<ButtonInput<MouseButton>>,
    mut capture: ResMut<PointerCapture>,
    state: Res<State<AppState>>,
    mut paused: ResMut<Paused>,
    pause_on_focus_loss: Res<PauseOnFocusLoss>,
) {
    // 휴대폰에서 홈 화면으로 나가거나 전화가 오면 앱이 멈추기 전에 게임을 일시정지한다.
    let suspending = lifecycle
        .read()
        .any(|event| matches!(event, AppLifecycle::WillSuspend | AppLifecycle::Suspended));
    if suspending && *state.get() == AppState::InGame && !paused.0 {
        paused.0 = true;
    }
    for event in events.read() {
        focus.0 = event.focused;
        if !event.focused {
            // 다른 창으로 넘어가면 게임을 멈춘다. 릴이 도는 채로 방치되지 않게 한다.
            if pause_on_focus_loss.0 && *state.get() == AppState::InGame && !paused.0 {
                paused.0 = true;
            }
            keys.release_all();
            keys.clear();
            buttons.release_all();
            buttons.clear();
            capture.0 = false;
        }
    }
}

fn keyboard_input(
    focus: Res<InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
    map: Option<Res<KeyMap>>,
    state: Res<State<AppState>>,
    paused: Res<Paused>,
    mut machine: MessageWriter<MachineCommand>,
    mut app: MessageWriter<AppCommand>,
) {
    let Some(map) = map else {
        return;
    };
    if !focus.0 {
        return;
    }
    let playing = *state.get() == AppState::InGame && !paused.0;
    for (key, action) in map.bindings() {
        if !keys.just_pressed(*key) {
            continue;
        }
        match action {
            Action::Machine(command) => {
                if playing {
                    machine.write(*command);
                }
            }
            Action::App(command) => {
                app.write(*command);
            }
        }
    }
}

/// 마우스·트랙패드로 게임기 부품을 누른다.
fn press_part(
    press: On<PointerPress>,
    focus: Res<InputFocus>,
    state: Res<State<AppState>>,
    paused: Res<Paused>,
    parts: Query<&Clickable>,
    buttons: Query<(), With<Button>>,
    mut capture: ResMut<PointerCapture>,
    mut machine: MessageWriter<MachineCommand>,
) {
    if press.button != PointerButton::Primary || !focus.0 {
        return;
    }
    // 화면 버튼을 누른 손가락이 움직여도 카메라가 돌지 않게 한다.
    if buttons.contains(press.entity) {
        capture.0 = true;
        return;
    }
    let Ok(clickable) = parts.get(press.entity) else {
        return;
    };
    capture.0 = true;
    if *state.get() == AppState::InGame && !paused.0 {
        machine.write(clickable.0);
    }
}

fn release_pointer(release: On<PointerRelease>, mut capture: ResMut<PointerCapture>) {
    if release.button == PointerButton::Primary {
        capture.0 = false;
    }
}

/// 빈 곳을 끌면 카메라가 돌고, 스크롤·핀치로 확대한다.
/// 손가락 드래그는 마우스보다 이동량이 작아서 회전 감도를 조금 올린다.
const TOUCH_ORBIT_SCALE: f32 = 1.4;

fn pointer_camera(
    focus: Res<InputFocus>,
    state: Res<State<AppState>>,
    paused: Res<Paused>,
    settings: Res<Settings>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut pinches: MessageReader<PinchGesture>,
    touches: Res<Touches>,
    mut last_spread: Local<Option<f32>>,
    mut capture: ResMut<PointerCapture>,
    mut controls: MessageWriter<CameraControl>,
) {
    let pinch: f32 = pinches.read().map(|gesture| gesture.0).sum();
    let fingers: Vec<&bevy::input::touch::Touch> = touches.iter().collect();
    if !buttons.pressed(MouseButton::Left) && fingers.is_empty() {
        capture.0 = false;
    }
    if fingers.len() != 2 {
        *last_spread = None;
    }
    if !focus.0 || *state.get() != AppState::InGame || paused.0 {
        return;
    }
    let mouse = &settings.config.controls.mouse;
    let mut control = CameraControl {
        orbit: Vec2::ZERO,
        zoom: 0.0,
    };
    let dragging =
        (buttons.pressed(MouseButton::Left) && !capture.0) || buttons.pressed(MouseButton::Right);
    if dragging {
        control.orbit = motion.delta * mouse.orbit_sensitivity;
    }
    if scroll.delta.y != 0.0 {
        // 트랙패드는 픽셀 단위, 마우스 휠은 줄 단위로 들어온다.
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        control.zoom += lines * mouse.zoom_sensitivity;
    }
    control.zoom += pinch;
    // 터치: 빈 곳을 한 손가락으로 끌면 회전, 두 손가락을 벌리거나 오므리면 확대·축소.
    match fingers.as_slice() {
        [finger] if !capture.0 => {
            control.orbit += finger.delta() * mouse.orbit_sensitivity * TOUCH_ORBIT_SCALE;
        }
        [a, b] => {
            let spread = a.position().distance(b.position());
            if let Some(previous) = *last_spread
                && previous > 1.0
            {
                control.zoom += spread / previous - 1.0;
            }
            *last_spread = Some(spread);
        }
        _ => {}
    }
    if control.orbit != Vec2::ZERO || control.zoom != 0.0 {
        controls.write(control);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn app(pause_on_focus_loss: bool) -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .add_message::<WindowFocused>()
            .add_message::<AppLifecycle>()
            .init_resource::<InputFocus>()
            .init_resource::<PointerCapture>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Paused>()
            .insert_resource(PauseOnFocusLoss(pause_on_focus_loss))
            .insert_state(AppState::InGame)
            .add_systems(Update, track_focus);
        app.update();
        app
    }

    fn lose_focus(app: &mut App) -> bool {
        app.world_mut().write_message(WindowFocused {
            window: Entity::PLACEHOLDER,
            focused: false,
        });
        app.update();
        assert!(!app.world().resource::<InputFocus>().0);
        app.world().resource::<Paused>().0
    }

    #[test]
    fn focus_loss_pauses_by_default() {
        assert!(lose_focus(&mut app(true)));
    }

    #[test]
    fn automated_runs_keep_playing_without_focus() {
        assert!(!lose_focus(&mut app(false)));
    }

    /// 트랙패드 핀치를 보낼 수 있는 플레이 상태의 앱.
    fn camera_app(paused: bool) -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin)
            .add_message::<PinchGesture>()
            .add_message::<CameraControl>()
            .init_resource::<PointerCapture>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .init_resource::<Touches>()
            .insert_resource(InputFocus(true))
            .insert_resource(Paused(paused))
            .insert_resource(Settings {
                config: fruit_star_config::ConfigSet::default(),
                root: std::path::PathBuf::new(),
            })
            .insert_state(AppState::InGame)
            .add_systems(Update, pointer_camera);
        app.update();
        app
    }

    fn pinch(app: &mut App, amount: f32) -> f32 {
        app.world_mut().write_message(PinchGesture(amount));
        app.update();
        app.world_mut()
            .resource_mut::<Messages<CameraControl>>()
            .drain()
            .map(|control| control.zoom)
            .sum()
    }

    #[test]
    fn trackpad_pinch_zooms_the_camera_in_and_out() {
        let mut app = camera_app(false);
        assert!(pinch(&mut app, 0.25) > 0.2);
        assert!(pinch(&mut app, -0.25) < -0.2);
    }

    #[test]
    fn trackpad_pinch_is_ignored_while_paused() {
        assert_eq!(pinch(&mut camera_app(true), 0.25), 0.0);
    }
}
