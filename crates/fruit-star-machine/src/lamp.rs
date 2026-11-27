//! LED와 램프의 점등 패턴 (논리 상태).

use bevy::prelude::*;
use fruit_star_game::{GameEvent, Phase};

use crate::messages::MachineEvent;
use crate::state::{Machine, Settings};

/// LED 띠의 위치.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LedStrip {
    Left,
    Right,
    Top,
}

/// 램프 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LampKind {
    /// 외곽 LED 띠의 한 알.
    Led {
        strip: LedStrip,
        index: u8,
        count: u8,
    },
    /// STAR CHANCE 램프.
    Chance,
    /// 릴 내부 조명.
    ReelBacklight(usize),
    /// 정지 버튼 조명.
    StopButton(usize),
    /// MAX BET 버튼 조명.
    BetButton,
    /// 상단 패널 백라이트.
    Marquee,
    /// 하단 장식 패널 백라이트.
    Belly,
    /// 당첨 라인 막대 ([`fruit_star_game::PAYLINES`]의 인덱스). 그 라인에서 당첨되면 켜진다.
    PayLine(u8),
    /// 유효 라인 표시등. 베팅으로 그 라인이 유효해지면 켜지고, 당첨되면 깜빡인다.
    LineLamp(u8),
    /// 1 BET 버튼 조명.
    BetOneButton,
    /// 게임기 꼭대기의 톱 램프. 보너스와 당첨을 주변에 알린다.
    TopLamp,
    /// 상태 표시등.
    Status(StatusLamp),
}

/// 릴 패널의 상태 표시등.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusLamp {
    /// 리플레이가 성립해 다음 게임이 무료다.
    Replay,
    /// 레버를 당길 수 있다.
    Start,
    /// 메달이 모자라 투입이 필요하다.
    Insert,
}

/// 램프 하나의 점등 상태. 렌더링 계층이 밝기와 색을 재질에 반영한다.
#[derive(Component, Debug, Clone, Copy)]
pub struct Lamp {
    pub kind: LampKind,
    /// 밝기 (0 = 소등, 1 = 기본 점등).
    pub brightness: f32,
    pub tint: Color,
}

impl Lamp {
    pub fn new(kind: LampKind) -> Self {
        Self {
            kind,
            brightness: 0.0,
            tint: Color::WHITE,
        }
    }
}

/// 게임기 전체의 조명 연출 모드.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LampMode {
    /// 대기 중의 느린 흐름.
    #[default]
    Attract,
    /// 릴 회전 중.
    Spin,
    /// 당첨 연출.
    Win,
    /// 보너스 게임 중.
    Bonus,
}

/// 보너스 시작 직후의 강조 연출 남은 시간 (초).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct Fanfare(pub f32);

/// STAR CHANCE 램프가 막 켜졌을 때의 번쩍임 남은 시간 (초).
#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct ChanceFlash(pub f32);

/// 번쩍임 길이.
const CHANCE_FLASH_S: f32 = 0.6;

/// STAR CHANCE 램프의 밝기. 켜지는 순간 짧게 번쩍인 뒤 꺼질 때까지 일정하게 밝다.
fn chance_brightness(lit: bool, flash_left: f32, t: f32, reduce: bool) -> f32 {
    if !lit {
        0.0
    } else if flash_left > 0.0 {
        if reduce {
            1.5
        } else if (t * 12.0).fract() < 0.5 {
            2.4
        } else {
            1.2
        }
    } else {
        1.2
    }
}

const GOLD: Color = Color::srgb(1.0, 0.78, 0.25);
const WARM_WHITE: Color = Color::srgb(1.0, 0.93, 0.8);
const AMBER: Color = Color::srgb(1.0, 0.55, 0.1);
const CHANCE_BLUE: Color = Color::srgb(0.45, 0.75, 1.0);

/// 0~1 범위의 부드러운 맥동.
fn pulse(t: f32, hz: f32) -> f32 {
    0.5 + 0.5 * (t * hz * std::f32::consts::TAU).sin()
}

/// 점멸. `reduce`가 참이면 주파수를 낮추고 깊이를 줄인 부드러운 변화로 바꾼다.
fn flash(t: f32, hz: f32, reduce: bool) -> f32 {
    if reduce {
        0.8 + 0.2 * pulse(t, hz.min(1.0))
    } else if (t * hz).fract() < 0.5 {
        1.0
    } else {
        0.25
    }
}

/// LED 띠 한 알의 (밝기, 색).
pub fn led_state(
    mode: LampMode,
    strip: LedStrip,
    index: u8,
    count: u8,
    t: f32,
    fanfare: bool,
    reduce: bool,
) -> (f32, Color) {
    let position = if count > 1 {
        f32::from(index) / f32::from(count)
    } else {
        0.0
    };
    // 좌우 띠는 아래에서 위로, 상단 띠는 가운데에서 바깥으로 흐른다.
    let phase = match strip {
        LedStrip::Left | LedStrip::Right => position,
        LedStrip::Top => (position - 0.5).abs() * 2.0,
    };
    // 흐르는 빛: 주기 안의 한 구간만 밝다.
    let chase = |speed: f32, width: f32| {
        let x = (phase - t * speed).rem_euclid(1.0);
        (1.0 - x / width).clamp(0.0, 1.0)
    };
    match mode {
        LampMode::Attract => {
            let speed = if reduce { 0.12 } else { 0.25 };
            (0.18 + 0.82 * chase(speed, 0.45), GOLD)
        }
        LampMode::Spin => {
            let speed = if reduce { 0.3 } else { 1.1 };
            (0.3 + 0.7 * chase(speed, 0.35), WARM_WHITE)
        }
        LampMode::Win => (flash(t, 5.0, reduce), GOLD),
        LampMode::Bonus => {
            let speed = if reduce { 0.1 } else { 0.5 };
            let hue = ((phase - t * speed).rem_euclid(1.0)) * 360.0;
            let brightness = if fanfare {
                flash(t, 6.0, reduce) * 1.2
            } else {
                0.75 + 0.25 * chase(speed * 2.0, 0.5)
            };
            (brightness, Color::hsl(hue, 1.0, 0.6))
        }
    }
}

pub(crate) fn update_lamp_mode(
    time: Res<Time>,
    machine: Res<Machine>,
    mut events: MessageReader<MachineEvent>,
    mut mode: ResMut<LampMode>,
    mut fanfare: ResMut<Fanfare>,
    mut chance_flash: ResMut<ChanceFlash>,
) {
    for MachineEvent(event) in events.read() {
        match event {
            GameEvent::BonusStarted(_) => fanfare.0 = 2.5,
            GameEvent::ChanceLampChanged(true) => chance_flash.0 = CHANCE_FLASH_S,
            _ => {}
        }
    }
    fanfare.0 = (fanfare.0 - time.delta_secs()).max(0.0);
    chance_flash.0 = (chance_flash.0 - time.delta_secs()).max(0.0);

    let won = machine
        .last_outcome()
        .is_some_and(|o| o.payout > 0 || o.replay_awarded || o.bonus_started.is_some());
    let next = match machine.phase() {
        Phase::Spinning | Phase::Stopping | Phase::Evaluating => LampMode::Spin,
        Phase::Result if won => LampMode::Win,
        _ if machine.bonus().is_some() => LampMode::Bonus,
        _ => LampMode::Attract,
    };
    if *mode != next {
        *mode = next;
    }
}

pub(crate) fn update_lamps(
    time: Res<Time>,
    machine: Res<Machine>,
    settings: Res<Settings>,
    mode: Res<LampMode>,
    fanfare: Res<Fanfare>,
    chance_flash: Res<ChanceFlash>,
    mut lamps: Query<&mut Lamp>,
) {
    let t = time.elapsed_secs_wrapped();
    let reduce = settings.config.graphics.led.reduce_flashing;
    let mode = *mode;
    let fanfare = fanfare.0 > 0.0;
    let ready = machine.phase() == Phase::Ready;
    let winning = mode == LampMode::Win;
    // 당첨 라인 표시는 결과가 나온 뒤부터 다음 게임의 레버를 당길 때까지 유지한다.
    let showing_result = matches!(machine.phase(), Phase::Result | Phase::Ready);
    let won_lines: Vec<u8> = machine
        .last_outcome()
        .filter(|_| showing_result)
        .map(|outcome| outcome.wins.iter().map(|win| win.line).collect())
        .unwrap_or_default();
    let has_medals = machine.mode().auto_refill() || machine.credit() >= 1;
    let can_bet = machine.can_add_bet() && has_medals;
    let lit_lines = machine.lit_lines();

    for mut lamp in &mut lamps {
        let (brightness, tint) = match lamp.kind {
            LampKind::Led {
                strip,
                index,
                count,
            } => led_state(mode, strip, index, count, t, fanfare, reduce),
            LampKind::Chance => (
                chance_brightness(machine.chance_lamp(), chance_flash.0, t, reduce),
                CHANCE_BLUE,
            ),
            LampKind::ReelBacklight(_) => {
                if winning {
                    (0.85 + 0.35 * flash(t, 4.0, reduce), WARM_WHITE)
                } else {
                    (1.0, WARM_WHITE)
                }
            }
            LampKind::StopButton(reel) => {
                if machine.can_stop(reel) {
                    (1.0, AMBER)
                } else {
                    (0.08, AMBER)
                }
            }
            LampKind::BetButton | LampKind::BetOneButton => {
                if can_bet && !machine.bet_placed() {
                    // 아직 아무것도 걸지 않았다: 눌러 달라고 맥동한다.
                    (0.55 + 0.45 * pulse(t, 1.0), GOLD)
                } else if can_bet {
                    // 일부만 걸려 있다: 더 걸 수 있음을 은은하게 알린다.
                    (0.6, GOLD)
                } else if machine.bet_placed() && ready {
                    (1.0, GOLD)
                } else {
                    (0.1, GOLD)
                }
            }
            LampKind::Marquee => match mode {
                LampMode::Bonus => (
                    0.95 + 0.25 * pulse(t, if reduce { 0.5 } else { 2.0 }),
                    WARM_WHITE,
                ),
                LampMode::Win => (1.15, WARM_WHITE),
                _ => (1.0, WARM_WHITE),
            },
            LampKind::TopLamp => match mode {
                LampMode::Bonus => (
                    0.5 + 0.7 * pulse(t, if reduce { 0.6 } else { 2.5 }),
                    Color::srgb(1.0, 0.25, 0.2),
                ),
                LampMode::Win => (flash(t, 4.0, reduce), Color::srgb(1.0, 0.25, 0.2)),
                _ if machine.chance_lamp() => (0.35 + 0.25 * pulse(t, 0.8), CHANCE_BLUE),
                _ => (0.04, Color::srgb(1.0, 0.25, 0.2)),
            },
            LampKind::Belly => match mode {
                LampMode::Bonus => (1.1, WARM_WHITE),
                _ => (0.85, WARM_WHITE),
            },
            LampKind::PayLine(line) => {
                if !won_lines.contains(&line) {
                    (0.0, GOLD)
                } else if machine.phase() == Phase::Result {
                    // 지급되는 동안은 깜빡이고, 끝나면 켜진 채로 남는다.
                    (flash(t, 3.0, reduce), GOLD)
                } else {
                    (0.85, GOLD)
                }
            }
            LampKind::LineLamp(line) => {
                if won_lines.contains(&line) {
                    if machine.phase() == Phase::Result {
                        (flash(t, 3.0, reduce), GOLD)
                    } else {
                        (1.0, GOLD)
                    }
                } else if usize::from(line) < lit_lines {
                    // 베팅으로 유효해진 라인
                    (0.45, AMBER)
                } else {
                    (0.0, AMBER)
                }
            }
            LampKind::Status(status) => {
                let on = match status {
                    StatusLamp::Replay => ready && machine.replay_pending(),
                    StatusLamp::Start => ready && machine.bet_placed(),
                    StatusLamp::Insert => ready && !machine.bet_placed() && !has_medals,
                };
                let color = match status {
                    StatusLamp::Replay => CHANCE_BLUE,
                    StatusLamp::Start => Color::srgb(0.45, 1.0, 0.5),
                    StatusLamp::Insert => Color::srgb(1.0, 0.35, 0.25),
                };
                if on {
                    (
                        0.75 + 0.25 * pulse(t, if reduce { 0.5 } else { 1.2 }),
                        color,
                    )
                } else {
                    (0.0, color)
                }
            }
        };
        lamp.brightness = brightness;
        lamp.tint = tint;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chance_lamp_flashes_when_lit_then_stays_steady() {
        assert_eq!(chance_brightness(false, CHANCE_FLASH_S, 0.0, false), 0.0);
        // 막 켜졌을 때는 평소보다 훨씬 밝게 번쩍인다
        let peak = (0..24)
            .map(|i| chance_brightness(true, 0.3, i as f32 / 48.0, false))
            .fold(0.0_f32, f32::max);
        assert!(peak >= 2.0, "{peak}");
        // 번쩍임이 끝나면 시간이 지나도 같은 밝기로 켜져 있다
        let steady: Vec<f32> = (0..10)
            .map(|i| chance_brightness(true, 0.0, i as f32 * 0.37, false))
            .collect();
        assert!(
            steady
                .iter()
                .all(|b| (*b - steady[0]).abs() < 1e-6 && *b > 1.0)
        );
        // 점멸 줄이기에서는 번쩍임 대신 한 번 밝아진다
        let calm: Vec<f32> = (0..24)
            .map(|i| chance_brightness(true, 0.3, i as f32 / 48.0, true))
            .collect();
        assert!(calm.iter().all(|b| (*b - calm[0]).abs() < 1e-6));
    }
}
