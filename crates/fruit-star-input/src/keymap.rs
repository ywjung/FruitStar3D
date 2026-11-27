//! 설정의 키 이름을 Bevy 키 코드와 동작으로 바꾼다.

use bevy::prelude::*;
use fruit_star_config::KeyBindings;
use fruit_star_machine::{AppCommand, MachineCommand};

/// 키에 배정되는 동작.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Machine(MachineCommand),
    App(AppCommand),
}

/// 키 → 동작 표.
#[derive(Resource, Debug, Clone, Default)]
pub struct KeyMap {
    bindings: Vec<(KeyCode, Action)>,
}

impl KeyMap {
    /// 설정으로부터 만든다. 알 수 없는 키 이름은 건너뛰고 목록으로 돌려준다.
    pub fn from_config(keys: &KeyBindings) -> (Self, Vec<String>) {
        let mut bindings = Vec::new();
        let mut unknown = Vec::new();
        for (name, bound) in keys.actions() {
            let Some(action) = action_for(name) else {
                continue;
            };
            for key in bound {
                match parse_key(key) {
                    Some(code) => bindings.push((code, action)),
                    None => unknown.push(key.clone()),
                }
            }
        }
        (Self { bindings }, unknown)
    }

    pub fn bindings(&self) -> &[(KeyCode, Action)] {
        &self.bindings
    }

    /// `key`에 배정된 동작.
    pub fn action(&self, key: KeyCode) -> Option<Action> {
        self.bindings
            .iter()
            .find(|(code, _)| *code == key)
            .map(|(_, action)| *action)
    }
}

/// 설정의 동작 이름에 해당하는 동작.
pub fn action_for(name: &str) -> Option<Action> {
    Some(match name {
        "bet" => Action::Machine(MachineCommand::Bet),
        "bet_one" => Action::Machine(MachineCommand::BetOne),
        "lever" => Action::Machine(MachineCommand::Lever),
        "stop_left" => Action::Machine(MachineCommand::Stop(0)),
        "stop_center" => Action::Machine(MachineCommand::Stop(1)),
        "stop_right" => Action::Machine(MachineCommand::Stop(2)),
        "insert_medal" => Action::Machine(MachineCommand::InsertMedal),
        "camera" => Action::App(AppCommand::CycleCamera),
        "fullscreen" => Action::App(AppCommand::ToggleFullscreen),
        "mute" => Action::App(AppCommand::ToggleMute),
        "pause" => Action::App(AppCommand::TogglePause),
        "menu" => Action::App(AppCommand::OpenMenu),
        "debug" => Action::App(AppCommand::ToggleDebug),
        _ => return None,
    })
}

/// W3C `KeyboardEvent.code` 표기의 키 이름을 키 코드로 바꾼다.
pub fn parse_key(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match name {
        "KeyA" => KeyA,
        "KeyB" => KeyB,
        "KeyC" => KeyC,
        "KeyD" => KeyD,
        "KeyE" => KeyE,
        "KeyF" => KeyF,
        "KeyG" => KeyG,
        "KeyH" => KeyH,
        "KeyI" => KeyI,
        "KeyJ" => KeyJ,
        "KeyK" => KeyK,
        "KeyL" => KeyL,
        "KeyM" => KeyM,
        "KeyN" => KeyN,
        "KeyO" => KeyO,
        "KeyP" => KeyP,
        "KeyQ" => KeyQ,
        "KeyR" => KeyR,
        "KeyS" => KeyS,
        "KeyT" => KeyT,
        "KeyU" => KeyU,
        "KeyV" => KeyV,
        "KeyW" => KeyW,
        "KeyX" => KeyX,
        "KeyY" => KeyY,
        "KeyZ" => KeyZ,
        "Digit0" => Digit0,
        "Digit1" => Digit1,
        "Digit2" => Digit2,
        "Digit3" => Digit3,
        "Digit4" => Digit4,
        "Digit5" => Digit5,
        "Digit6" => Digit6,
        "Digit7" => Digit7,
        "Digit8" => Digit8,
        "Digit9" => Digit9,
        "Space" => Space,
        "Enter" => Enter,
        "Escape" => Escape,
        "Tab" => Tab,
        "Backspace" => Backspace,
        "ArrowUp" => ArrowUp,
        "ArrowDown" => ArrowDown,
        "ArrowLeft" => ArrowLeft,
        "ArrowRight" => ArrowRight,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "Comma" => Comma,
        "Period" => Period,
        "Slash" => Slash,
        "Minus" => Minus,
        "Equal" => Equal,
        _ => return None,
    })
}
