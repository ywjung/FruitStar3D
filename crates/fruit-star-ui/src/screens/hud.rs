//! 플레이 계기판과 키 배정표. 키 이름은 입력 설정에서 읽는다.

use bevy::prelude::*;
use fruit_star_config::KeyBindings;
use fruit_star_machine::{AppCommand, MachineCommand};

use super::screen;
use crate::widgets::*;
use crate::{Dynamic, Group, Localized, Screen, UiAction};

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
/// 터치 조작판 두 줄의 높이와 간격, 판의 안쪽 여백.
const TOUCH_ROW_TOP: f32 = 50.0;
const TOUCH_ROW_STOP: f32 = 62.0;
const TOUCH_GAP: f32 = 8.0;
const DECK_PAD_TOP: f32 = 10.0;
const DECK_PAD_BOTTOM: f32 = SAFE_BOTTOM + 4.0;
/// 표시줄·조작판과 3D 화면 사이의 경계선 두께.
const EDGE: f32 = 3.0;
/// 하단 조작판 전체 높이 (UI 단위).
const DECK_HEIGHT: f32 =
    EDGE + DECK_PAD_TOP + TOUCH_ROW_TOP + TOUCH_GAP + TOUCH_ROW_STOP + DECK_PAD_BOTTOM;

/// 휴대폰 플레이 화면의 상단 표시줄. 카메라는 이것과 [`HudDeck`]이 덮지 않는 곳에 게임기를 둔다.
#[derive(Component)]
pub(crate) struct HudHeader;

/// 휴대폰 플레이 화면의 하단 조작판.
#[derive(Component)]
pub(crate) struct HudDeck;

/// 터치 조작판의 버튼 하나. 아랫면을 두껍게 칠해 누르는 단추처럼 보이게 한다.
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
                    border: UiRect::bottom(px(4)),
                    border_radius: BorderRadius::all(px(9)),
                    ..row(0.0)
                },
            ),
            BorderColor::all(finish.edge()),
        ))
        .with_children(|b| {
            b.spawn((
                label(
                    if height > TOUCH_ROW_TOP { 19.0 } else { 16.0 },
                    finish.ink(),
                    FontWeight::BOLD,
                ),
                Localized(key),
            ));
        });
}

/// 상단 표시줄의 작은 버튼 (시점 전환, 일시정지).
fn header_tool(parent: &mut ChildSpawnerCommands, key: &'static str, command: AppCommand) {
    parent
        .spawn(switch(
            UiAction::App(command),
            Finish::Dark,
            Node {
                height: px(36),
                padding: UiRect::axes(px(10), px(0)),
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..row(0.0)
            },
        ))
        .insert(BorderColor::all(Color::srgb_u8(92, 74, 70)))
        .with_children(|b| caption(b, key, 13.0, LIGHT));
}

/// 상단 표시줄의 값 하나: 작은 이름 아래 큰 숫자.
fn header_readout(parent: &mut ChildSpawnerCommands, key: &'static str, field: Dynamic) {
    parent
        .spawn((
            Node {
                min_width: px(48),
                align_items: AlignItems::Center,
                ..column(0.0)
            },
            Pickable::IGNORE,
        ))
        .with_children(|m| {
            caption(m, key, 11.0, DIM);
            value(m, field, 22.0, GOLD);
        });
}

/// 상단 표시줄: 안전 영역부터 화면 위 끝까지 채우고, 모드·크레딧·베팅·지급과 시점·일시정지를 한 줄에 둔다.
fn touch_header(root: &mut ChildSpawnerCommands) {
    root.spawn((
        HudHeader,
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            right: px(0),
            padding: UiRect {
                left: px(14),
                right: px(10),
                top: px(SAFE_TOP),
                bottom: px(7),
            },
            border: UiRect::bottom(px(EDGE)),
            justify_content: JustifyContent::SpaceBetween,
            ..row(8.0)
        },
        BackgroundColor(PANEL),
        BorderColor::all(RED),
        Pickable::IGNORE,
    ))
    .with_children(|bar| {
        bar.spawn((
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_width: px(0),
                ..column(1.0)
            },
            Pickable::IGNORE,
        ))
        .with_children(|title| {
            value(title, Dynamic::Mode, 14.0, LIGHT);
            value(title, Dynamic::Muted, 11.0, GOLD);
        });
        bar.spawn((Group::PlayOnly, row(6.0), Pickable::IGNORE))
            .with_children(|r| {
                header_readout(r, "hud.credit", Dynamic::Credit);
                header_readout(r, "hud.bet", Dynamic::Bet);
                header_readout(r, "hud.payout", Dynamic::Payout);
            });
        bar.spawn((row(6.0), Pickable::IGNORE))
            .with_children(|tools| {
                header_tool(tools, "touch.view", AppCommand::CycleCamera);
                header_tool(tools, "touch.pause", AppCommand::TogglePause);
            });
    });
}

/// 하단 조작판: 메달·베팅·레버 한 줄과 정지 버튼 한 줄. 화면 아래 끝까지 채운다.
/// 게임기의 버튼을 직접 눌러도 되지만, 휴대폰 화면에서는 부품이 작아서 이 판을 기본으로 쓴다.
fn touch_deck(root: &mut ChildSpawnerCommands) {
    root.spawn((
        HudDeck,
        Group::Touch,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            bottom: px(0),
            padding: UiRect {
                left: px(10),
                right: px(10),
                top: px(DECK_PAD_TOP),
                bottom: px(DECK_PAD_BOTTOM),
            },
            border: UiRect::top(px(EDGE)),
            ..column(TOUCH_GAP)
        },
        BackgroundColor(PANEL),
        BorderColor::all(RED),
        Pickable::IGNORE,
    ))
    .with_children(|panel| {
        panel
            .spawn((row(TOUCH_GAP), Pickable::IGNORE))
            .with_children(|r| {
                for (key, command, finish, grow) in [
                    (
                        "touch.medal",
                        MachineCommand::InsertMedal,
                        Finish::Dark,
                        1.0,
                    ),
                    ("touch.bet_one", MachineCommand::BetOne, Finish::Inset, 1.0),
                    ("touch.max_bet", MachineCommand::Bet, Finish::Paper, 1.15),
                    ("touch.start", MachineCommand::Lever, Finish::Enamel, 1.15),
                ] {
                    touch_key(r, key, command, finish, grow, TOUCH_ROW_TOP);
                }
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

/// 당첨·알림·안내 문구. 휴대폰에서는 하단 조작판 바로 위, 3D 화면 아래쪽에 띄운다.
fn messages(root: &mut ChildSpawnerCommands, touch: bool) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(if touch { 6.0 } else { 28.0 }),
            right: percent(if touch { 6.0 } else { 28.0 }),
            bottom: px(if touch { DECK_HEIGHT + 10.0 } else { 16.0 }),
            ..column(3.0)
        },
        Pickable::IGNORE,
    ))
    .with_children(|messages| {
        signal(messages, Dynamic::Banner, 21.0, GOLD);
        signal(messages, Dynamic::Notice, 15.0, LIGHT);
        signal(messages, Dynamic::Guide, 16.0, GOLD);
        if touch {
            // 키보드 화면에서는 왼쪽 위 계기판에 있는 연습 모드 안내.
            signal(messages, Dynamic::Practice, 14.0, LIGHT);
        }
    });
}

pub(super) fn spawn(commands: &mut Commands, keys: &KeyBindings, touch: bool) {
    commands
        .spawn(screen(
            Screen::Hud,
            AlignItems::Stretch,
            JustifyContent::SpaceBetween,
        ))
        .with_children(|root| {
            if touch {
                // 휴대폰: 위아래 막대가 3D 화면을 가리지 않도록 자리를 차지하고,
                // 카메라가 그 사이에 게임기를 둔다 (`HudInsets`).
                touch_header(root);
                messages(root, true);
                touch_deck(root);
            } else {
                desktop(root, keys);
            }
        });
}

/// 키보드·마우스 화면: 왼쪽 위 계기판, 아래쪽 조작표·도구 막대.
fn desktop(root: &mut ChildSpawnerCommands, keys: &KeyBindings) {
    root.spawn((
        Group::Readouts,
        Node {
            margin: UiRect {
                left: px(16),
                right: px(16),
                top: px(16),
                bottom: px(0),
            },
            ..column(0.0)
        },
        Pickable::IGNORE,
    ))
    .with_children(meters);
    messages(root, false);
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
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
                        toolbar_action(tools, "keys.camera", AppCommand::CycleCamera, &keys.camera);
                        toolbar_action(tools, "keys.pause", AppCommand::TogglePause, &keys.menu);
                    });
            });
    });
}
