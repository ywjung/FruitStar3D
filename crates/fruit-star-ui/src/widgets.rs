//! 간판, 인쇄 패널, 누름 버튼에 공통으로 쓰는 색과 조판.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::Button;

use crate::{Dynamic, Localized, UiAction, UiText};

pub const PAPER: Color = Color::srgb_u8(245, 236, 216);
pub const INK: Color = Color::srgb_u8(45, 35, 39);
pub const MUTED: Color = Color::srgb_u8(103, 83, 73);
pub const RULE: Color = Color::srgb_u8(204, 185, 158);
pub const RED: Color = Color::srgb_u8(156, 39, 52);
pub const GOLD: Color = Color::srgb_u8(255, 205, 112);
pub const METAL: Color = Color::srgb_u8(43, 40, 43);
pub const LIGHT: Color = Color::srgb_u8(251, 242, 224);

pub fn column(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(gap),
        ..default()
    }
}

pub fn row(gap: f32) -> Node {
    Node {
        align_items: AlignItems::Center,
        column_gap: px(gap),
        ..default()
    }
}

pub fn label(size: f32, color: Color, weight: FontWeight) -> impl Bundle {
    (
        Text::default(),
        TextFont {
            font_size: bevy::text::FontSize::Px(size),
            weight,
            ..default()
        },
        TextColor(color),
        UiText,
        Pickable::IGNORE,
    )
}

pub fn caption(parent: &mut ChildSpawnerCommands, key: &'static str, size: f32, color: Color) {
    parent.spawn((label(size, color, FontWeight::SEMIBOLD), Localized(key)));
}

pub fn value(parent: &mut ChildSpawnerCommands, field: Dynamic, size: f32, color: Color) {
    parent.spawn((label(size, color, FontWeight::BOLD), field));
}

pub fn rule(parent: &mut ChildSpawnerCommands, color: Color) {
    parent.spawn((
        Node {
            height: px(1),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(color),
        Pickable::IGNORE,
    ));
}

#[derive(Component, Clone, Copy)]
pub enum Finish {
    Enamel,
    Paper,
    Inset,
    Dark,
}

impl Finish {
    fn colors(self) -> [Color; 3] {
        match self {
            Self::Enamel => [
                RED,
                Color::srgb_u8(181, 50, 60),
                Color::srgb_u8(123, 28, 40),
            ],
            Self::Paper => [
                PAPER,
                Color::srgb_u8(235, 218, 190),
                Color::srgb_u8(220, 197, 167),
            ],
            Self::Inset => [
                Color::srgb_u8(232, 217, 192),
                Color::srgb_u8(222, 200, 170),
                Color::srgb_u8(212, 185, 151),
            ],
            Self::Dark => [
                METAL,
                Color::srgb_u8(71, 59, 59),
                Color::srgb_u8(94, 61, 60),
            ],
        }
    }
    pub fn ink(self) -> Color {
        match self {
            Self::Enamel | Self::Dark => LIGHT,
            _ => INK,
        }
    }
}

pub fn switch(action: UiAction, finish: Finish, node: Node) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        action,
        finish,
        node,
        BackgroundColor(finish.colors()[0]),
    )
}

pub fn button(
    parent: &mut ChildSpawnerCommands,
    key: &'static str,
    action: UiAction,
    finish: Finish,
) {
    parent
        .spawn(switch(
            action,
            finish,
            Node {
                min_height: px(36),
                padding: UiRect::axes(px(13), px(5)),
                justify_content: JustifyContent::Center,
                flex_grow: 1.0,
                ..row(0.0)
            },
        ))
        .with_children(|b| caption(b, key, 15.0, finish.ink()));
}

pub fn style_buttons(
    touch: Res<fruit_star_machine::TouchUi>,
    mut buttons: Query<(&Finish, &Hovered, Has<Pressed>, &mut BackgroundColor)>,
) {
    for (finish, hovered, pressed, mut color) in &mut buttons {
        // 터치에는 "올려 둔" 상태가 없다. 손을 뗀 뒤에도 남는 hover 표시는 무시한다.
        let hovered = Hovered(hovered.get() && !touch.0);
        let [idle, hover, down] = finish.colors();
        let wanted = if pressed {
            down
        } else if hovered.get() {
            hover
        } else {
            idle
        };
        if color.0 != wanted {
            color.0 = wanted;
        }
    }
}
