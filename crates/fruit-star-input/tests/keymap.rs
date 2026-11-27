#![allow(clippy::unwrap_used, clippy::expect_used)]

use bevy::prelude::KeyCode;
use fruit_star_config::{ControlsConfig, KNOWN_KEYS};
use fruit_star_input::{Action, KeyMap, parse_key};
use fruit_star_machine::{AppCommand, MachineCommand};

#[test]
fn every_known_key_name_parses_to_a_distinct_key() {
    let mut seen = Vec::new();
    for name in KNOWN_KEYS {
        let key = parse_key(name).unwrap_or_else(|| panic!("{name}을(를) 해석할 수 없다"));
        assert!(!seen.contains(&key), "{name} 중복");
        seen.push(key);
    }
    assert_eq!(parse_key("NoSuchKey"), None);
}

#[test]
fn default_bindings_match_the_documented_controls() {
    let (map, unknown) = KeyMap::from_config(&ControlsConfig::default().keys);
    assert!(unknown.is_empty());
    let expect = [
        (KeyCode::KeyB, Action::Machine(MachineCommand::Bet)),
        (KeyCode::KeyN, Action::Machine(MachineCommand::BetOne)),
        (KeyCode::Space, Action::Machine(MachineCommand::Lever)),
        (KeyCode::Digit1, Action::Machine(MachineCommand::Stop(0))),
        (KeyCode::Digit2, Action::Machine(MachineCommand::Stop(1))),
        (KeyCode::Digit3, Action::Machine(MachineCommand::Stop(2))),
        (KeyCode::KeyA, Action::Machine(MachineCommand::Stop(0))),
        (KeyCode::KeyS, Action::Machine(MachineCommand::Stop(1))),
        (KeyCode::KeyD, Action::Machine(MachineCommand::Stop(2))),
        (KeyCode::KeyC, Action::Machine(MachineCommand::InsertMedal)),
        (KeyCode::KeyV, Action::App(AppCommand::CycleCamera)),
        (KeyCode::KeyF, Action::App(AppCommand::ToggleFullscreen)),
        (KeyCode::KeyM, Action::App(AppCommand::ToggleMute)),
        (KeyCode::KeyP, Action::App(AppCommand::TogglePause)),
        (KeyCode::Escape, Action::App(AppCommand::OpenMenu)),
        (KeyCode::F3, Action::App(AppCommand::ToggleDebug)),
    ];
    for (key, action) in expect {
        assert_eq!(map.action(key), Some(action), "{key:?}");
    }
    assert_eq!(map.bindings().len(), expect.len());
    assert_eq!(map.action(KeyCode::KeyZ), None);
}

#[test]
fn unknown_key_names_are_reported_and_skipped() {
    let mut keys = ControlsConfig::default().keys;
    keys.bet = vec!["KeyB".to_owned(), "Hyper".to_owned()];
    let (map, unknown) = KeyMap::from_config(&keys);
    assert_eq!(unknown, vec!["Hyper".to_owned()]);
    assert_eq!(
        map.action(KeyCode::KeyB),
        Some(Action::Machine(MachineCommand::Bet))
    );
}
