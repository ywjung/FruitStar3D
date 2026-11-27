//! `controls.toml`: 키 배정과 마우스 감도.

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, check_range};

const FILE: &str = "controls.toml";

/// 설정에서 쓸 수 있는 키 이름 (W3C `KeyboardEvent.code` 표기).
pub const KNOWN_KEYS: &[&str] = &[
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Space",
    "Enter",
    "Escape",
    "Tab",
    "Backspace",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "Comma",
    "Period",
    "Slash",
    "Minus",
    "Equal",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KeyBindings {
    pub bet: Vec<String>,
    /// 1 BET (메달 한 장씩 베팅).
    pub bet_one: Vec<String>,
    pub lever: Vec<String>,
    pub stop_left: Vec<String>,
    pub stop_center: Vec<String>,
    pub stop_right: Vec<String>,
    pub insert_medal: Vec<String>,
    pub camera: Vec<String>,
    pub fullscreen: Vec<String>,
    pub mute: Vec<String>,
    pub pause: Vec<String>,
    pub menu: Vec<String>,
    /// 디버그 콘솔 (개발 빌드 전용).
    pub debug: Vec<String>,
}

fn keys(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            bet: keys(&["KeyB"]),
            bet_one: keys(&["KeyN"]),
            lever: keys(&["Space"]),
            stop_left: keys(&["Digit1", "KeyA"]),
            stop_center: keys(&["Digit2", "KeyS"]),
            stop_right: keys(&["Digit3", "KeyD"]),
            insert_medal: keys(&["KeyC"]),
            camera: keys(&["KeyV"]),
            fullscreen: keys(&["KeyF"]),
            mute: keys(&["KeyM"]),
            pause: keys(&["KeyP"]),
            menu: keys(&["Escape"]),
            debug: keys(&["F3"]),
        }
    }
}

impl KeyBindings {
    /// (동작 이름, 배정된 키 목록) 전체.
    pub fn actions(&self) -> [(&'static str, &[String]); 13] {
        [
            ("bet", &self.bet),
            ("bet_one", &self.bet_one),
            ("lever", &self.lever),
            ("stop_left", &self.stop_left),
            ("stop_center", &self.stop_center),
            ("stop_right", &self.stop_right),
            ("insert_medal", &self.insert_medal),
            ("camera", &self.camera),
            ("fullscreen", &self.fullscreen),
            ("mute", &self.mute),
            ("pause", &self.pause),
            ("menu", &self.menu),
            ("debug", &self.debug),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MouseSection {
    /// 드래그 1픽셀당 카메라 회전량 (라디안).
    pub orbit_sensitivity: f32,
    /// 스크롤·핀치 한 단위당 확대 비율.
    pub zoom_sensitivity: f32,
}

impl Default for MouseSection {
    fn default() -> Self {
        Self {
            orbit_sensitivity: 0.005,
            zoom_sensitivity: 0.08,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControlsConfig {
    pub keys: KeyBindings,
    pub mouse: MouseSection,
}

impl ControlsConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut used: Vec<(&str, &str)> = Vec::new();
        for (action, bound) in self.keys.actions() {
            if bound.is_empty() && action != "debug" {
                return Err(ConfigError::invalid(
                    FILE,
                    format!("keys.{action}"),
                    "키가 하나 이상 배정되어야 합니다",
                ));
            }
            for key in bound {
                if !KNOWN_KEYS.contains(&key.as_str()) {
                    return Err(ConfigError::invalid(
                        FILE,
                        format!("keys.{action}"),
                        format!("알 수 없는 키 이름 '{key}'"),
                    ));
                }
                if let Some((other, _)) = used.iter().find(|(_, k)| *k == key.as_str()) {
                    return Err(ConfigError::invalid(
                        FILE,
                        format!("keys.{action}"),
                        format!("'{key}' 키가 {other}에도 배정되어 있습니다"),
                    ));
                }
                used.push((action, key.as_str()));
            }
        }
        check_range(
            FILE,
            "mouse.orbit_sensitivity",
            self.mouse.orbit_sensitivity,
            0.000_1,
            0.1,
        )?;
        check_range(
            FILE,
            "mouse.zoom_sensitivity",
            self.mouse.zoom_sensitivity,
            0.001,
            1.0,
        )?;
        Ok(())
    }
}
