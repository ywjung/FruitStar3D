//! 화면 구성. 메뉴와 설정은 캐비닛의 인쇄 패널, HUD는 조작부의 계기판을 따른다.

mod hud;
pub(crate) use hud::key_names;

use bevy::prelude::*;
use fruit_star_game::PlayMode;
use fruit_star_machine::{Settings, TouchUi, ViewMode};

use crate::widgets::*;
use crate::{Dynamic, Group, Localized, Screen, SettingKind, StatKind, UiAction};

fn screen(kind: Screen, align: AlignItems, justify: JustifyContent) -> impl Bundle {
    (
        kind,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: align,
            justify_content: justify,
            display: Display::None,
            ..column(0.0)
        },
        Pickable::IGNORE,
    )
}

fn marquee(parent: &mut ChildSpawnerCommands, image: Handle<Image>) {
    parent.spawn((
        ImageNode::new(image),
        Node {
            width: percent(100),
            aspect_ratio: Some(960.0 / 336.0),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
}

#[derive(Component)]
pub(super) enum MenuPart {
    Rail,
    Mode,
}

fn menu_mode(
    parent: &mut ChildSpawnerCommands,
    name: &'static str,
    hint: &'static str,
    mode: PlayMode,
) {
    let finish = if mode == PlayMode::Classic {
        Finish::Enamel
    } else {
        Finish::Paper
    };
    parent
        .spawn(switch(
            UiAction::Start(mode, ViewMode::Play),
            finish,
            Node {
                justify_content: JustifyContent::SpaceBetween,

                ..row(8.0)
            },
        ))
        .insert(MenuPart::Mode)
        .with_children(|b| {
            b.spawn((column(1.0), Pickable::IGNORE))
                .with_children(|copy| {
                    caption(copy, name, 22.0, finish.ink());
                    let ink = if mode == PlayMode::Classic {
                        LIGHT
                    } else {
                        MUTED
                    };
                    copy.spawn((label(14.0, ink, FontWeight::NORMAL), Localized(hint)));
                });
            b.spawn((
                Text::new("›"),
                TextFont::from_font_size(28.0),
                TextColor(finish.ink()),
                Pickable::IGNORE,
            ));
        });
}

fn modes(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((Group::Modes, column(0.0)))
        .with_children(|body| {
            caption(body, "menu.play_modes", 15.0, RED);
            rule(body, RULE);
            menu_mode(body, "menu.classic", "hint.classic", PlayMode::Classic);
            menu_mode(body, "menu.free_play", "hint.free_play", PlayMode::FreePlay);
            rule(body, RULE);
            menu_mode(body, "menu.practice", "hint.practice", PlayMode::Practice);
            rule(body, RULE);
            body.spawn(Node {
                margin: UiRect::top(px(20)),
                ..column(8.0)
            })
            .with_children(|view| {
                caption(view, "menu.views", 15.0, RED);
                view.spawn(row(6.0)).with_children(|buttons| {
                    button(
                        buttons,
                        "menu.mechanical",
                        UiAction::Start(PlayMode::FreePlay, ViewMode::Mechanical),
                        Finish::Inset,
                    );
                    button(
                        buttons,
                        "menu.gallery",
                        UiAction::Start(PlayMode::FreePlay, ViewMode::Gallery),
                        Finish::Inset,
                    );
                });
            });
        });
}

fn records(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((Group::Records, column(6.0)))
        .with_children(|body| {
            caption(body, "menu.records", 25.0, INK);
            rule(body, RULE);
            for (key, field) in [
                ("stats.games", StatKind::Games),
                ("stats.in_out", StatKind::InOut),
                ("stats.bonus", StatKind::Bonus),
                ("stats.time", StatKind::Time),
                ("stats.achievements", StatKind::Achievements),
            ] {
                body.spawn(Node {
                    min_height: px(30),
                    justify_content: JustifyContent::SpaceBetween,
                    ..row(12.0)
                })
                .with_children(|r| {
                    caption(r, key, 16.0, MUTED);
                    value(r, Dynamic::Stat(field), 19.0, INK);
                });
            }
            rule(body, RULE);
            body.spawn(switch(
                UiAction::ResetHistory,
                Finish::Inset,
                Node {
                    padding: UiRect::all(px(8)),
                    justify_content: JustifyContent::Center,
                    ..row(0.0)
                },
            ))
            .with_children(|b| value(b, Dynamic::ResetLabel, 15.0, RED));
        });
}

fn menu(commands: &mut Commands, image: Handle<Image>, machine_name: &str, touch: bool) {
    commands
        .spawn(screen(
            Screen::Menu,
            AlignItems::FlexStart,
            JustifyContent::Center,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    height: percent(100),

                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::right(px(5)),
                    ..column(0.0)
                },
                MenuPart::Rail,
                BackgroundColor(PAPER),
                BorderColor::all(RED),
            ))
            .with_children(|rail| {
                rail.spawn(column(9.0)).with_children(|head| {
                    marquee(head, image);
                    caption(head, "subtitle", 14.0, MUTED);
                });
                modes(rail);
                records(rail);
                rail.spawn(column(6.0)).with_children(|footer| {
                    value(footer, Dynamic::Notice, 15.0, RED);
                    value(footer, Dynamic::StorageStatus, 13.0, MUTED);
                    rule(footer, RULE);
                    footer.spawn(row(3.0)).with_children(|bar| {
                        bar.spawn(switch(
                            UiAction::ToggleRecords,
                            Finish::Paper,
                            Node {
                                min_height: px(36),
                                padding: UiRect::horizontal(px(9)),
                                flex_grow: 1.0,
                                justify_content: JustifyContent::Center,
                                ..row(0.0)
                            },
                        ))
                        .with_children(|b| value(b, Dynamic::RecordsLabel, 15.0, RED));
                        button(bar, "menu.language", UiAction::Language, Finish::Paper);
                        // iOS 앱은 스스로 종료하지 않는다 (홈 화면으로 나가면 된다).
                        if !touch {
                            button(bar, "menu.quit", UiAction::Quit, Finish::Paper);
                        }
                    });
                });
            });
            root.spawn((
                // 세로 화면에서는 메뉴가 화면 전체를 덮어 이 표시가 간판과 겹친다.
                Group::DesktopOnly,
                Node {
                    position_type: PositionType::Absolute,
                    right: px(24),
                    top: px(20),
                    align_items: AlignItems::FlexEnd,
                    ..column(0.0)
                },
                Pickable::IGNORE,
            ))
            .with_children(|plate| {
                caption(plate, "menu.machine", 13.0, LIGHT);
                plate.spawn((
                    Text::new(machine_name),
                    TextFont {
                        weight: FontWeight::BOLD,
                        ..TextFont::from_font_size(24.0).with_family("Avenir Next Condensed")
                    },
                    TextColor(LIGHT),
                    Pickable::IGNORE,
                ));
            });
        });
}

fn setting(parent: &mut ChildSpawnerCommands, key: &'static str, kind: SettingKind) {
    rule(parent, RULE);
    parent
        .spawn(Node {
            min_height: px(41),
            justify_content: JustifyContent::SpaceBetween,
            ..row(12.0)
        })
        .with_children(|r| {
            caption(r, key, 16.0, INK);
            let toggle = matches!(kind, SettingKind::ReduceFlashing | SettingKind::Quality);
            if toggle {
                r.spawn(switch(
                    UiAction::Adjust(kind, 1),
                    Finish::Inset,
                    Node {
                        width: px(126),
                        min_height: px(32),
                        justify_content: JustifyContent::Center,
                        ..row(0.0)
                    },
                ))
                .with_children(|b| value(b, Dynamic::Setting(kind), 16.0, RED));
            } else {
                r.spawn(row(0.0)).with_children(|dial| {
                    let step = |dial: &mut ChildSpawnerCommands, delta, symbol: &'static str| {
                        dial.spawn(switch(
                            UiAction::Adjust(kind, delta),
                            Finish::Inset,
                            Node {
                                width: px(35),
                                height: px(32),
                                justify_content: JustifyContent::Center,
                                ..row(0.0)
                            },
                        ))
                        .with_child((
                            Text::new(symbol),
                            TextFont::from_font_size(20.0),
                            TextColor(INK),
                            Pickable::IGNORE,
                        ));
                    };
                    step(dial, -1, "−");
                    dial.spawn(Node {
                        width: px(56),
                        justify_content: JustifyContent::Center,
                        ..row(0.0)
                    })
                    .with_children(|val| value(val, Dynamic::Setting(kind), 16.0, RED));
                    step(dial, 1, "+");
                });
            }
        });
}

fn pause(commands: &mut Commands, touch: bool) {
    commands
        .spawn((
            screen(Screen::Pause, AlignItems::Center, JustifyContent::Center),
            BackgroundColor(Color::srgba(0.08, 0.06, 0.07, 0.74)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(488),
                    max_width: percent(100),
                    border: UiRect::top(px(6)),
                    padding: UiRect::axes(px(26), px(22)),
                    ..column(0.0)
                },
                BackgroundColor(PAPER),
                BorderColor::all(RED),
            ))
            .with_children(|panel| {
                panel
                    .spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::bottom(px(18)),
                        ..row(10.0)
                    })
                    .with_children(|head| {
                        caption(head, "pause.title", 32.0, INK);
                        value(head, Dynamic::Mode, 16.0, RED);
                    });
                for (group, entries) in [
                    (
                        "settings.sound",
                        [
                            ("settings.master", SettingKind::Master),
                            ("settings.effects", SettingKind::Effects),
                            ("settings.ambient", SettingKind::Ambient),
                        ],
                    ),
                    (
                        "settings.picture",
                        [
                            ("settings.led", SettingKind::Led),
                            ("settings.reduce", SettingKind::ReduceFlashing),
                            ("settings.quality", SettingKind::Quality),
                        ],
                    ),
                ] {
                    panel
                        .spawn(Node {
                            margin: UiRect::vertical(px(7)),
                            ..column(0.0)
                        })
                        .with_children(|heading| caption(heading, group, 14.0, RED));
                    for (key, kind) in entries {
                        setting(panel, key, kind);
                    }
                    // 휴대폰은 무음 모드에서 게임 소리가 나지 않는다. 앱은 무음 모드를 알 수 없으므로 안내만 한다.
                    if touch && group == "settings.sound" {
                        panel
                            .spawn(Node {
                                margin: UiRect::vertical(px(6)),
                                ..column(0.0)
                            })
                            .with_children(|note| caption(note, "settings.silent_hint", 13.0, INK));
                    }
                }
                rule(panel, RULE);
                panel
                    .spawn(Node {
                        margin: UiRect::top(px(20)),
                        ..row(6.0)
                    })
                    .with_children(|bar| {
                        button(bar, "menu.language", UiAction::Language, Finish::Inset);
                        button(bar, "menu.to_menu", UiAction::ToMenu, Finish::Inset);
                        button(bar, "menu.resume", UiAction::Resume, Finish::Enamel);
                    });
            });
        });
}

pub(crate) fn spawn_screens(
    mut commands: Commands,
    assets: Res<AssetServer>,
    settings: Res<Settings>,
    touch: Res<TouchUi>,
) {
    let image = assets.load("textures/menu_marquee.png");
    commands
        .spawn((
            screen(Screen::Loading, AlignItems::Center, JustifyContent::Center),
            BackgroundColor(PAPER),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: px(360),
                ..column(18.0)
            })
            .with_children(|p| {
                marquee(p, image.clone());
                rule(p, RULE);
                caption(p, "loading", 18.0, RED);
            });
        });
    commands
        .spawn((
            screen(Screen::Failed, AlignItems::Center, JustifyContent::Center),
            BackgroundColor(INK),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(540),
                    // 휴대폰 세로 화면은 540보다 좁다.
                    max_width: percent(92),
                    padding: UiRect::all(px(24)),
                    ..column(16.0)
                },
                BackgroundColor(PAPER),
            ))
            .with_children(|p| {
                caption(p, "failed.title", 28.0, RED);
                value(p, Dynamic::Failure, 17.0, INK);
                // iOS 앱은 스스로 종료하지 않으므로 종료 단추를 두지 않는다 (메인 메뉴와 같다).
                if !touch.0 {
                    button(p, "menu.quit", UiAction::Quit, Finish::Enamel);
                }
            });
        });
    menu(&mut commands, image, &settings.config.machine.name, touch.0);
    hud::spawn(&mut commands, &settings.config.controls.keys, touch.0);
    pause(&mut commands, touch.0);
    commands
        .spawn(screen(
            Screen::Debug,
            AlignItems::FlexEnd,
            JustifyContent::FlexStart,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    padding: UiRect::all(px(12)),
                    margin: UiRect::all(px(12)),
                    ..default()
                },
                BackgroundColor(METAL),
                Pickable::IGNORE,
            ))
            .with_children(|p| value(p, Dynamic::Debug, 13.0, LIGHT));
        });
}
