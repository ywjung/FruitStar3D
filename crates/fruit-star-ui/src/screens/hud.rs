//! 플레이 계기판과 키 배정표. 키 이름은 입력 설정에서 읽는다.

use bevy::prelude::*;
use fruit_star_config::KeyBindings;
use fruit_star_machine::{AppCommand, MachineCommand};

use super::screen;
use crate::widgets::*;
use crate::{Dynamic, Group, Screen, UiAction};

pub(crate) fn key_names(keys: &[String]) -> String {
    keys.iter()
        .map(|raw| {
            let key = raw
                .strip_prefix("Key")
                .or_else(|| raw.strip_prefix("Digit"))
                .unwrap_or(raw);
            match key {
                "Escape" => "Esc",
                "ArrowLeft" => "←",
                "ArrowRight" => "→",
                "ArrowUp" => "↑",
                "ArrowDown" => "↓",
                "Comma" => ",",
                "Period" => ".",
                "Slash" => "/",
                "Minus" => "−",
                "Equal" => "=",
                _ => key,
            }
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

fn keycap(parent: &mut ChildSpawnerCommands, keys: &[String]) {
    parent
        .spawn((
            Node {
                min_width: px(31),
                padding: UiRect::axes(px(7), px(1)),
                justify_content: JustifyContent::Center,
                ..row(0.0)
            },
            BackgroundColor(PAPER),
            Pickable::IGNORE,
        ))
        .with_child((
            Text::new(key_names(keys)),
            TextFont::from_font_size(14.0),
            TextColor(INK),
            Pickable::IGNORE,
        ));
}

fn meters(root: &mut ChildSpawnerCommands) {
    root.spawn((
        Node {
            align_self: AlignSelf::FlexStart,
            max_width: px(296),
            padding: UiRect::axes(px(16), px(10)),
            border: UiRect::top(px(3)),
            ..column(4.0)
        },
        BackgroundColor(METAL),
        BorderColor::all(RED),
        Pickable::IGNORE,
    ))
    .with_children(|p| {
        p.spawn((
            Node {
                justify_content: JustifyContent::SpaceBetween,
                ..row(16.0)
            },
            Pickable::IGNORE,
        ))
        .with_children(|head| {
            value(head, Dynamic::Mode, 16.0, LIGHT);
            value(head, Dynamic::Muted, 13.0, GOLD);
        });
        p.spawn((Group::PlayOnly, row(16.0), Pickable::IGNORE))
            .with_children(|r| {
                for (key, field) in [
                    ("hud.credit", Dynamic::Credit),
                    ("hud.bet", Dynamic::Bet),
                    ("hud.payout", Dynamic::Payout),
                ] {
                    r.spawn((
                        Node {
                            min_width: px(46),
                            ..column(0.0)
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|m| {
                        caption(m, key, 13.0, LIGHT);
                        value(m, field, 26.0, GOLD);
                    });
                }
            });
        value(p, Dynamic::Practice, 14.0, LIGHT);
    });
}

fn binding(parent: &mut ChildSpawnerCommands, label: &'static str, keys: &[String]) {
    parent
        .spawn((
            Node {
                min_height: px(25),
                justify_content: JustifyContent::SpaceBetween,
                ..row(12.0)
            },
            Pickable::IGNORE,
        ))
        .with_children(|r| {
            caption(r, label, 14.0, LIGHT);
            keycap(r, keys);
        });
}

fn help(parent: &mut ChildSpawnerCommands, k: &KeyBindings) {
    parent
        .spawn((
            Group::Controls,
            Node {
                width: px(286),
                padding: UiRect::axes(px(14), px(10)),
                border: UiRect::top(px(3)),
                ..column(2.0)
            },
            BackgroundColor(METAL),
            BorderColor::all(GOLD),
            Pickable::IGNORE,
        ))
        .with_children(|p| {
            p.spawn((Group::PlayOnly, column(2.0), Pickable::IGNORE))
                .with_children(|rows| {
                    for (label, keys) in [
                        ("keys.max_bet", &k.bet),
                        ("keys.one_bet", &k.bet_one),
                        ("keys.start", &k.lever),
                        ("keys.left", &k.stop_left),
                        ("keys.center", &k.stop_center),
                        ("keys.right", &k.stop_right),
                        ("keys.medals", &k.insert_medal),
                    ] {
                        binding(rows, label, keys);
                    }
                });
            caption(p, "keys.orbit", 14.0, LIGHT);
            binding(p, "keys.camera", &k.camera);
            binding(p, "keys.fullscreen", &k.fullscreen);
            binding(p, "keys.mute", &k.mute);
            let pause_keys: Vec<_> = k.pause.iter().chain(&k.menu).cloned().collect();
            binding(p, "keys.pause", &pause_keys);
        });
}

fn signal(parent: &mut ChildSpawnerCommands, field: Dynamic, size: f32, ink: Color) {
    parent.spawn((
        label(size, ink, FontWeight::BOLD),
        field,
        Node {
            padding: UiRect::axes(px(12), px(5)),
            align_self: AlignSelf::Center,
            ..default()
        },
        BackgroundColor(METAL),
        TextLayout::justify(Justify::Center),
    ));
}

fn toolbar_action(
    parent: &mut ChildSpawnerCommands,
    title: &'static str,
    command: AppCommand,
    keys: &[String],
) {
    parent
        .spawn(switch(
            UiAction::App(command),
            Finish::Dark,
            Node {
                min_height: px(36),
                padding: UiRect::axes(px(11), px(5)),
                ..row(7.0)
            },
        ))
        .with_children(|b| {
            if let Some(first) = keys.first() {
                keycap(b, std::slice::from_ref(first));
            }
            caption(b, title, 15.0, LIGHT);
        });
}

/// 휴대폰 화면의 안전 영역 여백 (UI 단위). 위는 카메라 섬·노치, 아래는 홈 표시줄을 피한다.
const SAFE_TOP: f32 = 62.0;
const SAFE_BOTTOM: f32 = 26.0;
/// 터치 조작판 두 줄의 높이와 간격.
const TOUCH_ROW_TOP: f32 = 54.0;
const TOUCH_ROW_STOP: f32 = 72.0;
const TOUCH_GAP: f32 = 8.0;

/// 터치 조작판의 버튼 하나.
fn touch_key(
    parent: &mut ChildSpawnerCommands,
    key: &'static str,
    command: MachineCommand,
    finish: Finish,
    grow: f32,
    height: f32,
) {
    parent
        .spawn((
            // 게임기 버튼처럼 누르는 순간 동작한다. 손을 뗄 때 동작하면 빠르게 연달아
            // 누를 때 입력이 빠질 수 있고, 정지 버튼은 누른 시점이 중요하다.
            bevy::ui_widgets::ActivateOnPress,
            switch(
                UiAction::Machine(command),
                finish,
                Node {
                    height: px(height),
                    flex_grow: grow,
                    flex_basis: px(0),
                    justify_content: JustifyContent::Center,
                    ..row(0.0)
                },
            ),
        ))
        .with_children(|b| caption(b, key, 17.0, finish.ink()));
}

/// 터치 조작판: 화면 아래에 크게 놓인 메달·베팅·레버·정지 버튼.
/// 게임기의 버튼을 직접 눌러도 되지만, 휴대폰 화면에서는 부품이 작아서 이 판을 기본으로 쓴다.
fn touch_panel(root: &mut ChildSpawnerCommands, bottom: f32) {
    root.spawn((
        Group::Touch,
        Node {
            position_type: PositionType::Absolute,
            left: px(10),
            right: px(10),
            bottom: px(bottom),
            ..column(TOUCH_GAP)
        },
        Pickable::IGNORE,
    ))
    .with_children(|panel| {
        panel
            .spawn((row(TOUCH_GAP), Pickable::IGNORE))
            .with_children(|r| {
                touch_key(
                    r,
                    "touch.medal",
                    MachineCommand::InsertMedal,
                    Finish::Dark,
                    1.0,
                    TOUCH_ROW_TOP,
                );
                touch_key(
                    r,
                    "touch.bet_one",
                    MachineCommand::BetOne,
                    Finish::Inset,
                    1.0,
                    TOUCH_ROW_TOP,
                );
                touch_key(
                    r,
                    "touch.max_bet",
                    MachineCommand::Bet,
                    Finish::Paper,
                    1.25,
                    TOUCH_ROW_TOP,
                );
                touch_key(
                    r,
                    "touch.start",
                    MachineCommand::Lever,
                    Finish::Enamel,
                    1.5,
                    TOUCH_ROW_TOP,
                );
            });
        panel
            .spawn((row(TOUCH_GAP), Pickable::IGNORE))
            .with_children(|r| {
                for reel in 0..3 {
                    touch_key(
                        r,
                        "touch.stop",
                        MachineCommand::Stop(reel),
                        Finish::Enamel,
                        1.0,
                        TOUCH_ROW_STOP,
                    );
                }
            });
    });
}

pub(super) fn spawn(commands: &mut Commands, keys: &KeyBindings, touch: bool) {
    // 터치 환경에서는 안전 영역을 비우고, 아래쪽에 터치 조작판을 둔다.
    let (top, bottom) = if touch {
        (SAFE_TOP, SAFE_BOTTOM)
    } else {
        (16.0, 12.0)
    };
    let bar_height = 36.0;
    let touch_bottom = bottom + bar_height + 10.0;
    let messages_bottom = if touch {
        touch_bottom + TOUCH_ROW_TOP + TOUCH_ROW_STOP + TOUCH_GAP + 14.0
    } else {
        16.0
    };
    let no_keys: &[String] = &[];
    commands
        .spawn(screen(
            Screen::Hud,
            AlignItems::Stretch,
            JustifyContent::SpaceBetween,
        ))
        .with_children(|root| {
            root.spawn((
                Group::Readouts,
                Node {
                    margin: UiRect {
                        left: px(16),
                        right: px(16),
                        top: px(top),
                        bottom: px(0),
                    },
                    ..column(0.0)
                },
                Pickable::IGNORE,
            ))
            .with_children(meters);
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(if touch { 6.0 } else { 28.0 }),
                    right: percent(if touch { 6.0 } else { 28.0 }),
                    bottom: px(messages_bottom),
                    ..column(3.0)
                },
                Pickable::IGNORE,
            ))
            .with_children(|messages| {
                signal(messages, Dynamic::Banner, 21.0, GOLD);
                signal(messages, Dynamic::Notice, 15.0, LIGHT);
                signal(messages, Dynamic::Guide, 16.0, GOLD);
            });
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(bottom),
                    left: px(16),
                    right: px(16),
                    ..column(6.0)
                },
                Pickable::IGNORE,
            ))
            .with_children(|bottom| {
                help(bottom, keys);
                bottom
                    .spawn((
                        Node {
                            justify_content: JustifyContent::SpaceBetween,
                            ..row(12.0)
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|bar| {
                        bar.spawn((row(10.0), Pickable::IGNORE))
                            .with_children(|left| {
                                left.spawn((
                                    Group::DesktopOnly,
                                    switch(
                                        UiAction::ToggleControls,
                                        Finish::Dark,
                                        Node {
                                            min_height: px(34),
                                            padding: UiRect::axes(px(12), px(4)),
                                            ..row(0.0)
                                        },
                                    ),
                                ))
                                .with_children(|b| value(b, Dynamic::HelpLabel, 15.0, LIGHT));
                                left.spawn((
                                    Group::GalleryTip,
                                    Node {
                                        padding: UiRect::axes(px(10), px(5)),
                                        ..row(0.0)
                                    },
                                    BackgroundColor(METAL),
                                    Pickable::IGNORE,
                                ))
                                .with_children(|tip| caption(tip, "keys.orbit", 14.0, LIGHT));
                            });
                        bar.spawn((row(4.0), Pickable::IGNORE))
                            .with_children(|tools| {
                                toolbar_action(
                                    tools,
                                    "keys.camera",
                                    AppCommand::CycleCamera,
                                    if touch { no_keys } else { &keys.camera },
                                );
                                toolbar_action(
                                    tools,
                                    "keys.pause",
                                    AppCommand::TogglePause,
                                    if touch { no_keys } else { &keys.menu },
                                );
                            });
                    });
            });
            if touch {
                touch_panel(root, touch_bottom);
            }
        });
}
