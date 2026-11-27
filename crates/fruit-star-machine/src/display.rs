//! CREDIT / PAYOUT / COUNT 7세그먼트 표시기 (논리 상태).

use bevy::prelude::*;

use fruit_star_game::GameEvent;

use crate::medal::PayoutCounter;
use crate::messages::MachineEvent;
use crate::state::Machine;

/// 표시기 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayKind {
    Credit,
    Payout,
    Count,
    /// 데이터 카운터: 마지막 보너스 이후의 게임 수.
    Games,
    /// 데이터 카운터: BIG 보너스 횟수.
    Big,
    /// 데이터 카운터: REGULAR 보너스 횟수.
    Regular,
    /// 데이터 카운터: 총 게임 수.
    Total,
}

impl DisplayKind {
    pub const ALL: [DisplayKind; 7] = [
        DisplayKind::Credit,
        DisplayKind::Payout,
        DisplayKind::Count,
        DisplayKind::Games,
        DisplayKind::Big,
        DisplayKind::Regular,
        DisplayKind::Total,
    ];

    /// 자릿수.
    pub const fn digits(self) -> u8 {
        match self {
            DisplayKind::Credit => 4,
            DisplayKind::Payout => 2,
            DisplayKind::Count => 3,
            DisplayKind::Games | DisplayKind::Total => 4,
            DisplayKind::Big | DisplayKind::Regular => 2,
        }
    }
}

/// 표시기에 나타낼 값.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DisplayValues {
    pub credit: u32,
    pub payout: u32,
    /// 보너스 중 남은 지급 가능 메달 수. 보너스가 아니면 `None`(소등).
    pub count: Option<u32>,
    /// 데이터 카운터 값.
    pub data: DataCounter,
}

impl DisplayValues {
    pub fn value(&self, kind: DisplayKind) -> Option<u32> {
        match kind {
            DisplayKind::Credit => Some(self.credit),
            DisplayKind::Payout => Some(self.payout),
            DisplayKind::Count => self.count,
            DisplayKind::Games => Some(self.data.since_bonus),
            DisplayKind::Big => Some(self.data.big),
            DisplayKind::Regular => Some(self.data.regular),
            DisplayKind::Total => Some(self.data.total_games),
        }
    }
}

/// 데이터 카운터: 게임기 위에 달린 집계 표시기의 값. 게임기 앞에 앉은 뒤부터 센다.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DataCounter {
    /// 총 게임 수.
    pub total_games: u32,
    /// 마지막 보너스가 끝난 뒤의 게임 수 (보너스 게임은 세지 않는다).
    pub since_bonus: u32,
    pub big: u32,
    pub regular: u32,
}

pub(crate) fn track_data_counter(
    mut events: MessageReader<MachineEvent>,
    mut counter: ResMut<DataCounter>,
) {
    for MachineEvent(event) in events.read() {
        match event {
            GameEvent::PowerOn => *counter = DataCounter::default(),
            GameEvent::GameCompleted(outcome) => {
                counter.total_games += 1;
                if !outcome.in_bonus {
                    counter.since_bonus += 1;
                }
                match outcome.bonus_started {
                    Some(fruit_star_game::BonusKind::Big) => {
                        counter.big += 1;
                        counter.since_bonus = 0;
                    }
                    Some(fruit_star_game::BonusKind::Regular) => {
                        counter.regular += 1;
                        counter.since_bonus = 0;
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }
}

/// 7세그먼트 한 획. 렌더링 계층이 `lit`에 따라 재질을 바꾼다.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SevenSegment {
    pub display: DisplayKind,
    /// 자리 (0 = 일의 자리).
    pub digit: u8,
    /// 획 번호 (0~6 = a~g).
    pub segment: u8,
    pub lit: bool,
}

/// 숫자별 점등 획 비트마스크 (비트 0 = a ... 비트 6 = g).
const DIGIT_SEGMENTS: [u8; 10] = [
    0b0111111, // 0
    0b0000110, // 1
    0b1011011, // 2
    0b1001111, // 3
    0b1100110, // 4
    0b1101101, // 5
    0b1111101, // 6
    0b0000111, // 7
    0b1111111, // 8
    0b1101111, // 9
];

/// `value`를 `digits`자리 표시기에 표시할 때 `digit`자리 `segment`획이 켜지는지 여부.
/// 앞자리 0은 끄고, 자릿수를 넘는 값은 최댓값(9…9)으로 표시한다.
pub fn segment_lit(value: Option<u32>, digits: u8, digit: u8, segment: u8) -> bool {
    let Some(value) = value else {
        return false;
    };
    if digit >= digits || segment >= 7 {
        return false;
    }
    let max = 10u32.saturating_pow(u32::from(digits)) - 1;
    let value = value.min(max);
    let place = 10u32.pow(u32::from(digit));
    if digit > 0 && value < place {
        return false;
    }
    let number = (value / place) % 10;
    DIGIT_SEGMENTS[number as usize] & (1 << segment) != 0
}

pub(crate) fn update_display_values(
    machine: Res<Machine>,
    counter: Res<PayoutCounter>,
    data: Res<DataCounter>,
    mut values: ResMut<DisplayValues>,
) {
    let next = DisplayValues {
        // 지급 연출이 끝나지 않은 메달은 아직 크레딧에 더해 보이지 않는다.
        credit: machine.credit().saturating_sub(counter.pending),
        payout: counter.paid,
        count: machine.bonus().map(|bonus| bonus.remaining()),
        data: *data,
    };
    if *values != next {
        *values = next;
    }
}

pub(crate) fn update_segments(values: Res<DisplayValues>, mut segments: Query<&mut SevenSegment>) {
    for mut segment in &mut segments {
        let lit = segment_lit(
            values.value(segment.display),
            segment.display.digits(),
            segment.digit,
            segment.segment,
        );
        if segment.lit != lit {
            segment.lit = lit;
        }
    }
}
