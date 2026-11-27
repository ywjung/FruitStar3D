//! FRUIT STAR 3D UI 계층: 메뉴, HUD, 일시정지·설정, 알림, 디버그 표시.
//!
//! 게임기 자체의 표시(7세그먼트, 램프)는 3D 캐비닛에 있고, 여기서는 그 위에 얹는
//! 화면 UI만 다룬다. 모든 문자열은 [`i18n`]의 번역표에서 가져온다.

pub mod i18n;
mod screens;
mod widgets;

use bevy::diagnostic::{
    DiagnosticsStore, FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use fruit_star_config::Language;
use fruit_star_game::{BonusKind, Flag, Phase, PlayMode};
use fruit_star_machine::{
    AppCommand, AppState, DebugLevel, DisplayValues, FailureMessage, HudInsets, Lamp, LampKind,
    Machine, MachineCommand, MachineSet, MainCamera, Notice, Paused, SessionCommand, Settings,
    StatsSnapshot, TouchUi, UiClick, ViewMode, overall_state,
};

use crate::i18n::{font_family, tr};

/// UI 플러그인.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        if !app.is_plugin_added::<SystemInformationDiagnosticsPlugin>() {
            app.add_plugins(SystemInformationDiagnosticsPlugin);
        }
        app.init_resource::<NoticeState>()
            .init_resource::<ResetArmed>()
            .init_resource::<Panels>()
            .init_resource::<CompactLayout>()
            .add_systems(OnEnter(AppState::MainMenu), reset_panels)
            .add_systems(Startup, screens::spawn_screens)
            .add_systems(
                Update,
                (
                    toggle_debug,
                    receive_notices,
                    fit_interface,
                    show_screens,
                    show_groups,
                    localize,
                    update_dynamic,
                    widgets::style_buttons,
                    publish_hud_insets,
                )
                    .chain()
                    .in_set(MachineSet::Present),
            )
            .add_observer(on_activate);
    }
}

/// 화면 단위의 최상위 노드.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    Loading,
    Failed,
    Menu,
    Hud,
    Pause,
    Debug,
}

/// 번역표의 고정 문자열을 표시하는 텍스트.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Localized(pub &'static str);

/// 상태에 따라 내용이 바뀌는 텍스트.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dynamic {
    Failure,
    Mode,
    Credit,
    Bet,
    Payout,
    Muted,
    Guide,
    HelpLabel,
    Practice,
    Banner,
    Stat(StatKind),
    StorageStatus,
    Notice,
    RecordsLabel,
    ResetLabel,
    Setting(SettingKind),
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatKind {
    Games,
    InOut,
    Bonus,
    Time,
    Achievements,
}

#[derive(Resource, Default)]
struct Panels {
    records: bool,
    controls: bool,
}

#[derive(Component, Clone, Copy)]
pub(crate) enum Group {
    Modes,
    Records,
    Controls,
    Readouts,
    PlayOnly,
    GalleryTip,
    /// 터치 조작판 (휴대폰에서 게임기 앞에 있을 때).
    Touch,
    /// 키보드·마우스 환경에서만 보이는 것 (조작표 열기 버튼 등).
    DesktopOnly,
}

/// 첫 카메라 크기가 결정될 때까지는 배치를 적용하지 않는다.
#[derive(Resource, Default)]
struct CompactLayout(Option<bool>);

fn reset_panels(mut panels: ResMut<Panels>, mut armed: ResMut<ResetArmed>) {
    *panels = Panels::default();
    armed.0 = 0.0;
}

/// 일시정지 화면에서 조절하는 설정.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingKind {
    Master,
    Effects,
    Ambient,
    Led,
    ReduceFlashing,
    Quality,
}

/// 버튼을 눌렀을 때의 동작.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiAction {
    Start(PlayMode, ViewMode),
    Language,
    ToggleRecords,
    ToggleControls,
    App(AppCommand),
    /// 화면의 터치 버튼으로 게임기를 조작한다.
    Machine(MachineCommand),
    ResetHistory,
    Quit,
    Resume,
    ToMenu,
    Adjust(SettingKind, i8),
}

/// 글꼴을 언어에 맞춰 바꿔야 하는 텍스트 표시.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct UiText;

/// 화면 아래에 잠깐 보이는 알림.
#[derive(Resource, Debug, Clone, Default)]
struct NoticeState {
    key: Option<String>,
    remaining: f32,
}

/// 기록 초기화 확인 대기 시간 (초). 0보다 크면 한 번 더 누를 때 실행한다.
#[derive(Resource, Debug, Clone, Copy, Default)]
struct ResetArmed(f32);

fn toggle_debug(mut commands: MessageReader<AppCommand>, mut level: ResMut<DebugLevel>) {
    for command in commands.read() {
        // 디버그 표시는 개발 빌드에서만 켤 수 있다.
        if *command == AppCommand::ToggleDebug && cfg!(debug_assertions) {
            level.0 = (level.0 + 1) % 3;
        }
    }
}

fn receive_notices(
    time: Res<Time<Real>>,
    mut notices: MessageReader<Notice>,
    mut state: ResMut<NoticeState>,
    mut armed: ResMut<ResetArmed>,
) {
    for notice in notices.read() {
        state.key = Some(notice.key.clone());
        state.remaining = 3.5;
    }
    if state.remaining > 0.0 {
        state.remaining -= time.delta_secs();
        if state.remaining <= 0.0 {
            state.key = None;
        }
    }
    if armed.0 > 0.0 {
        armed.0 = (armed.0 - time.delta_secs()).max(0.0);
    }
}

fn show_screens(
    state: Res<State<AppState>>,
    paused: Res<Paused>,
    level: Res<DebugLevel>,
    mut screens: Query<(&Screen, &mut Node)>,
) {
    let app = *state.get();
    for (screen, mut node) in &mut screens {
        let visible = match screen {
            Screen::Loading => app == AppState::Loading,
            Screen::Failed => app == AppState::Failed,
            Screen::Menu => app == AppState::MainMenu,
            Screen::Hud => app == AppState::InGame && !paused.0,
            Screen::Pause => app == AppState::InGame && paused.0,
            Screen::Debug => level.0 > 0,
        };
        let display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

/// 카메라의 논리 크기를 사용해서 Retina 창과 화면 밖 렌더링에 같은 배치를 적용한다.
fn fit_interface(
    camera: Single<&Camera, With<MainCamera>>,
    mut scale: ResMut<UiScale>,
    mut layout: ResMut<CompactLayout>,
    mut menu: Query<(&screens::MenuPart, &mut Node)>,
) {
    let Some(size) = camera.logical_viewport_size() else {
        return;
    };
    // 세로로 긴 화면(휴대폰)은 폭 420단위를 기준으로 맞춘다. 가로 화면은 높이를 기준으로 하되,
    // 작은 창은 메뉴 여백부터 줄인다. 설정과 조작표도 세로 500단위 안에 들어온다.
    let portrait = size.x < size.y;
    let fit = if portrait {
        (size.x / 420.0).clamp(0.72, 3.0)
    } else {
        (size.y / 720.0)
            .clamp(0.72, 1.5)
            .min(size.y / 500.0)
            .min(size.x / 840.0)
    };
    if fit <= 0.0 {
        return;
    }
    let wanted = ((fit * 32.0).floor() / 32.0).max(1.0 / 32.0);
    if scale.0 != wanted {
        scale.0 = wanted;
    }
    let compact = portrait || size.y / wanted < 600.0;
    if layout.0 != Some(compact) {
        layout.0 = Some(compact);
        for (part, mut node) in &mut menu {
            match part {
                screens::MenuPart::Rail => {
                    node.width = if portrait {
                        percent(100)
                    } else {
                        px(if compact { 338 } else { 400 })
                    };
                    node.padding = if portrait {
                        // 휴대폰: 위는 카메라 섬, 아래는 홈 표시줄을 피한다.
                        UiRect {
                            left: px(15),
                            right: px(15),
                            top: px(58),
                            bottom: px(24),
                        }
                    } else if compact {
                        UiRect::axes(px(15), px(9))
                    } else {
                        UiRect::axes(px(28), px(22))
                    };
                    node.row_gap = px(if compact { 4 } else { 12 });
                }
                screens::MenuPart::Mode => {
                    node.min_height = px(if compact { 49 } else { 67 });
                    node.padding = UiRect::axes(px(12), px(if compact { 2 } else { 6 }));
                }
            }
        }
    }
}

/// 휴대폰 플레이 화면의 상단 표시줄·하단 조작판이 실제로 덮는 높이를 카메라에 알린다.
///
/// 배치가 끝난 실제 크기(직전 프레임)를 재므로 글꼴·배율·안전 영역이 바뀌어도 맞는다.
/// 일시정지 화면처럼 계기판 전체가 숨으면 크기가 0이 되는데, 그때는 직전 값을 두어 화면이 튀지 않게 한다.
fn publish_hud_insets(
    state: Res<State<AppState>>,
    camera: Single<&Camera, With<MainCamera>>,
    header: Query<&ComputedNode, With<screens::HudHeader>>,
    deck: Query<&ComputedNode, With<screens::HudDeck>>,
    mut insets: ResMut<HudInsets>,
) {
    let next = if *state.get() == AppState::InGame {
        let Some(height) = camera
            .physical_viewport_size()
            .map(|size| size.y as f32)
            .filter(|height| *height > 0.0)
        else {
            return;
        };
        let covered = |nodes: &mut dyn Iterator<Item = &ComputedNode>| -> f32 {
            nodes.map(|node| node.size().y).fold(0.0, f32::max) / height
        };
        let next = HudInsets {
            top: covered(&mut header.iter()),
            bottom: covered(&mut deck.iter()),
        };
        if next.top <= 0.0 && next.bottom <= 0.0 {
            return;
        }
        next
    } else {
        HudInsets::default()
    };
    if *insets != next {
        *insets = next;
    }
}

fn show_groups(
    panels: Res<Panels>,
    view: Res<ViewMode>,
    layout: Res<CompactLayout>,
    touch: Res<TouchUi>,
    mut nodes: Query<(&Group, &mut Node)>,
) {
    if !panels.is_changed() && !view.is_changed() && !layout.is_changed() && !touch.is_changed() {
        return;
    }
    for (group, mut node) in &mut nodes {
        let show = match group {
            Group::Modes => !panels.records,
            Group::Records => panels.records,
            Group::Controls => panels.controls,
            Group::Readouts => !(layout.0 == Some(true) && panels.controls),
            Group::PlayOnly => view.playable(),
            Group::GalleryTip => !view.playable() && !panels.controls,
            Group::Touch => touch.0 && view.playable(),
            Group::DesktopOnly => !touch.0,
        };
        node.display = if show { Display::Flex } else { Display::None };
    }
}

/// 고정 문자열과 글꼴을 현재 언어에 맞춘다.
fn localize(
    settings: Res<Settings>,
    mut last: Local<Option<Language>>,
    mut texts: Query<(&Localized, &mut Text)>,
    mut fonts: Query<&mut TextFont, With<UiText>>,
    added: Query<(), Added<UiText>>,
) {
    let language = settings.config.application.game.language;
    if *last == Some(language) && added.is_empty() {
        return;
    }
    *last = Some(language);
    for (localized, mut text) in &mut texts {
        let value = tr(language, localized.0);
        if text.0 != value {
            text.0 = value.to_owned();
        }
    }
    let family = FontSource::family(font_family(language));
    for mut font in &mut fonts {
        font.font = family.clone();
    }
}

fn flag_key(flag: Flag) -> &'static str {
    match flag {
        Flag::Blank => "flag.blank",
        Flag::Bonus(BonusKind::Big) => "flag.big",
        Flag::Bonus(BonusKind::Regular) => "flag.regular",
        Flag::Small(role) => match role.key() {
            "replay" => "flag.replay",
            "grape" => "flag.grape",
            "cherry" => "flag.cherry",
            "bell" => "flag.bell",
            "melon" => "flag.melon",
            "star" => "flag.star",
            _ => "flag.blank",
        },
    }
}

/// 성립한 역할의 이름 번역 키.
fn role_key(role: fruit_star_game::Role) -> &'static str {
    use fruit_star_game::Role;
    match role {
        Role::Replay => "flag.replay",
        Role::Grape => "flag.grape",
        Role::Cherry => "flag.cherry",
        Role::Bell => "flag.bell",
        Role::Melon => "flag.melon",
        Role::Star => "flag.star",
        Role::BigRed => "role.big_red",
        Role::BigBlue => "role.big_blue",
        Role::Regular => "role.regular",
    }
}

fn percent_text(value: f32) -> String {
    format!("{:.0}%", value * 100.0)
}

fn update_dynamic(
    settings: Res<Settings>,
    machine: Res<Machine>,
    view: Res<ViewMode>,
    state: Res<State<AppState>>,
    paused: Res<Paused>,
    display: Res<DisplayValues>,
    stats: Res<StatsSnapshot>,
    failure: Res<FailureMessage>,
    notice: Res<NoticeState>,
    armed: Res<ResetArmed>,
    level: Res<DebugLevel>,
    diagnostics: Res<DiagnosticsStore>,
    lamps: Query<&Lamp>,
    panels: Res<Panels>,
    touch: Res<TouchUi>,
    mut texts: Query<(&Dynamic, &mut Text, &mut Node)>,
) {
    let config = &settings.config;
    let language = config.application.game.language;
    let t = |key: &str| tr(language, key).to_owned();
    let playable = view.playable();

    for (dynamic, mut text, mut node) in &mut texts {
        let value = match dynamic {
            Dynamic::Failure => failure.0.clone(),
            Dynamic::Mode => {
                let mode = t(&format!("mode.{}", machine.mode().key()));
                match *view {
                    ViewMode::Play => mode,
                    ViewMode::Mechanical => format!("{mode} · {}", t("view.mechanical")),
                    ViewMode::Gallery => t("view.gallery"),
                }
            }
            Dynamic::Credit => display.credit.to_string(),
            Dynamic::Bet => machine.displayed_bet().to_string(),
            Dynamic::Payout => display.payout.to_string(),
            Dynamic::Muted => {
                if config.audio.muted {
                    t("hud.muted")
                } else {
                    String::new()
                }
            }
            Dynamic::Guide => {
                if playable
                    && machine.phase() == Phase::Ready
                    && !machine.bet_placed()
                    && !machine.mode().auto_refill()
                    && machine.credit() == 0
                {
                    if touch.0 {
                        // 터치 화면에는 키보드가 없다. 아래 조작판의 메달 버튼을 누르면 된다.
                        t("hud.need_medals")
                    } else {
                        format!(
                            "{}   [{}]",
                            t("hud.need_medals"),
                            screens::key_names(&config.controls.keys.insert_medal)
                        )
                    }
                } else {
                    String::new()
                }
            }
            Dynamic::HelpLabel => t(if panels.controls {
                "hud.hide_keys"
            } else {
                "hud.show_keys"
            }),
            Dynamic::Practice => match machine.practice_hint() {
                Some((flag, carried)) if playable => {
                    let carried = match carried {
                        Some(kind) => t(flag_key(Flag::Bonus(kind))),
                        None => t("flag.none"),
                    };
                    format!(
                        "{}: {}   {}: {}",
                        t("practice.flag"),
                        t(flag_key(flag)),
                        t("practice.carried"),
                        carried
                    )
                }
                _ => String::new(),
            },
            Dynamic::Banner => {
                // 직전 게임의 당첨 내용. 다음 게임의 레버를 당기면 사라진다.
                let win = machine
                    .last_outcome()
                    .filter(|o| {
                        matches!(machine.phase(), Phase::Result | Phase::Ready)
                            && !o.wins.is_empty()
                    })
                    .map(|outcome| {
                        let mut names: Vec<String> = Vec::new();
                        for win in &outcome.wins {
                            let name = t(role_key(win.role));
                            if !names.contains(&name) {
                                names.push(name);
                            }
                        }
                        let mut line = format!("{} {}", names.join(" · "), t("hud.win"));
                        if outcome.payout > 0 {
                            line.push_str(&format!("  +{}", outcome.payout));
                        }
                        line
                    });
                if let Some(bonus) = machine.bonus() {
                    let title = match bonus.kind {
                        BonusKind::Big => t("bonus.big"),
                        BonusKind::Regular => t("bonus.regular"),
                    };
                    let status = format!("{title}   {} {}", t("hud.bonus_left"), bonus.remaining());
                    match win {
                        Some(win) => format!("{status}\n{win}"),
                        None => status,
                    }
                } else {
                    win.unwrap_or_default()
                }
            }
            Dynamic::Stat(kind) => match kind {
                StatKind::Games => stats.games.to_string(),
                StatKind::InOut => format!("{} / {}", stats.total_bet, stats.total_payout),
                StatKind::Bonus => format!("{} / {}", stats.big_bonuses, stats.regular_bonuses),
                StatKind::Time => format!(
                    "{}h {:02}m",
                    stats.play_seconds / 3600,
                    stats.play_seconds % 3600 / 60
                ),
                StatKind::Achievements => stats.achievements.len().to_string(),
            },
            Dynamic::StorageStatus => {
                if stats.persistent {
                    String::new()
                } else {
                    t("stats.not_saved")
                }
            }
            Dynamic::Notice => notice.key.as_deref().map(&t).unwrap_or_default(),
            Dynamic::RecordsLabel => t(if panels.records {
                "menu.back"
            } else {
                "menu.records"
            }),
            Dynamic::ResetLabel => {
                if armed.0 > 0.0 {
                    t("menu.reset_confirm")
                } else {
                    t("menu.reset")
                }
            }
            Dynamic::Setting(kind) => match kind {
                SettingKind::Master => percent_text(config.audio.master),
                SettingKind::Effects => percent_text(config.audio.effects),
                SettingKind::Ambient => percent_text(config.audio.ambient),
                SettingKind::Led => percent_text(config.graphics.led.brightness),
                SettingKind::ReduceFlashing => {
                    if config.graphics.led.reduce_flashing {
                        t("on")
                    } else {
                        t("off")
                    }
                }
                SettingKind::Quality => config.graphics.render.quality.key().to_uppercase(),
            },
            Dynamic::Debug => {
                if level.0 == 0 {
                    String::new()
                } else {
                    debug_text(
                        &diagnostics,
                        &machine,
                        &lamps,
                        overall_state(*state.get(), paused.0, Some(&machine)),
                        level.0,
                    )
                }
            }
        };
        let display = if value.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
        if text.0 != value {
            text.0 = value;
        }
    }
}

/// 디버그 콘솔 내용: FPS, 프레임 시간, 메모리, 릴 각도, LED 밝기.
fn debug_text(
    diagnostics: &DiagnosticsStore,
    machine: &Machine,
    lamps: &Query<&Lamp>,
    state: fruit_star_core::GameState,
    level: u8,
) -> String {
    let smoothed = |path: &bevy::diagnostic::DiagnosticPath| {
        diagnostics
            .get(path)
            .and_then(|diagnostic| diagnostic.smoothed())
    };
    let fps = smoothed(&FrameTimeDiagnosticsPlugin::FPS).unwrap_or(0.0);
    let frame_ms = smoothed(&FrameTimeDiagnosticsPlugin::FRAME_TIME).unwrap_or(0.0);
    let memory = diagnostics
        .get(&SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE)
        .and_then(|diagnostic| {
            diagnostic
                .value()
                .map(|v| (v, diagnostic.suffix.to_string()))
        });
    let mut out = format!("FPS {fps:.1}   frame {frame_ms:.2} ms\n");
    if let Some((value, suffix)) = memory {
        out.push_str(&format!("memory {value:.2} {suffix}\n"));
    }
    out.push_str(&format!(
        "state {state:?}   game #{}\n",
        machine.next_game_id()
    ));
    for reel in machine.reels() {
        out.push_str(&format!(
            "reel {}: {:>7.2}°  {:>6.1} rad/s  {:?}\n",
            reel.id(),
            reel.current_angle().to_degrees(),
            reel.angular_velocity(),
            reel.state(),
        ));
    }
    let (mut sum, mut count, mut peak) = (0.0f32, 0u32, 0.0f32);
    for lamp in lamps.iter() {
        if matches!(lamp.kind, LampKind::Led { .. }) {
            sum += lamp.brightness;
            count += 1;
            peak = peak.max(lamp.brightness);
        }
    }
    if count > 0 {
        out.push_str(&format!(
            "LED avg {:.2}  peak {:.2}  ({count})\n",
            sum / count as f32,
            peak
        ));
    }
    if level >= 2 {
        out.push_str("wireframe + colliders ON\n");
    }
    out
}

fn step(value: f32, delta: i8, max: f32) -> f32 {
    ((value + f32::from(delta) * 0.1) * 10.0)
        .round()
        .clamp(0.0, max * 10.0)
        / 10.0
}

fn on_activate(
    activate: On<Activate>,
    actions: Query<&UiAction>,
    mut settings: ResMut<Settings>,
    mut paused: ResMut<Paused>,
    mut armed: ResMut<ResetArmed>,
    mut sessions: MessageWriter<SessionCommand>,
    mut clicks: MessageWriter<UiClick>,
    mut panels: ResMut<Panels>,
    mut app: MessageWriter<AppCommand>,
    mut machine: MessageWriter<MachineCommand>,
) {
    let Ok(action) = actions.get(activate.entity) else {
        return;
    };
    // 게임기 버튼은 게임기 자체의 버튼 소리가 나므로 화면 선택음은 내지 않는다.
    if let UiAction::Machine(command) = *action {
        machine.write(command);
        return;
    }
    clicks.write(UiClick);
    match *action {
        UiAction::Start(mode, view) => {
            sessions.write(SessionCommand::Start { mode, view });
        }
        UiAction::Language => {
            let next = settings.config.application.game.language.next();
            settings.config.application.game.language = next;
        }
        UiAction::ToggleRecords => {
            panels.records = !panels.records;
            armed.0 = 0.0;
        }
        UiAction::ToggleControls => panels.controls = !panels.controls,
        UiAction::App(command) => {
            app.write(command);
        }
        UiAction::Machine(_) => {}
        UiAction::ResetHistory => {
            if armed.0 > 0.0 {
                armed.0 = 0.0;
                sessions.write(SessionCommand::ResetHistory);
            } else {
                armed.0 = 4.0;
            }
        }
        UiAction::Quit => {
            sessions.write(SessionCommand::Quit);
        }
        UiAction::Resume => paused.0 = false,
        UiAction::ToMenu => {
            sessions.write(SessionCommand::End);
        }
        UiAction::Adjust(kind, delta) => {
            let config = &mut settings.config;
            match kind {
                SettingKind::Master => config.audio.master = step(config.audio.master, delta, 1.0),
                SettingKind::Effects => {
                    config.audio.effects = step(config.audio.effects, delta, 1.0);
                }
                SettingKind::Ambient => {
                    config.audio.ambient = step(config.audio.ambient, delta, 1.0);
                }
                SettingKind::Led => {
                    config.graphics.led.brightness =
                        step(config.graphics.led.brightness, delta, 1.5);
                }
                SettingKind::ReduceFlashing => {
                    config.graphics.led.reduce_flashing = !config.graphics.led.reduce_flashing;
                }
                SettingKind::Quality => {
                    config.graphics.render.quality = config.graphics.render.quality.next();
                }
            }
        }
    }
}
