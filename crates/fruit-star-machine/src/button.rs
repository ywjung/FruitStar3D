//! 레버와 버튼의 눌림 애니메이션 (논리 상태).

use bevy::prelude::*;

use crate::messages::PartPressed;

/// 눌러서 움직이는 부품.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartKind {
    Lever,
    Bet,
    BetOne,
    Stop(usize),
    MedalSlot,
}

/// 눌림 애니메이션 상태. 렌더링 계층이 `amount`를 부품의 변위로 바꾼다.
#[derive(Component, Debug, Clone, Copy)]
pub struct Pressable {
    pub kind: PartKind,
    /// 애니메이션 전체 시간 (초).
    pub duration: f32,
    /// 진행 시간 (초). `duration` 이상이면 쉬는 상태다.
    pub elapsed: f32,
    /// 눌린 정도 (0 = 원위치, 1 = 끝까지 눌림). 복귀 때 살짝 음수가 될 수 있다.
    pub amount: f32,
}

impl Pressable {
    pub fn new(kind: PartKind, duration_ms: u32) -> Self {
        let duration = duration_ms as f32 / 1000.0;
        Self {
            kind,
            duration,
            elapsed: duration,
            amount: 0.0,
        }
    }

    pub fn is_animating(&self) -> bool {
        self.elapsed < self.duration
    }

    /// 애니메이션을 처음부터 다시 시작한다.
    pub fn trigger(&mut self) {
        self.elapsed = 0.0;
    }
}

/// 진행률 `t`(0~1)에서의 눌림 정도.
///
/// 앞 35%는 빠르게 눌리고 나머지는 스프링처럼 돌아오며, 레버는 복귀 끝에
/// 살짝 튕긴다.
pub fn press_curve(kind: PartKind, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const DOWN: f32 = 0.35;
    if t < DOWN {
        // ease-out: 손이 누르는 동작
        let x = t / DOWN;
        return 1.0 - (1.0 - x) * (1.0 - x);
    }
    let x = (t - DOWN) / (1.0 - DOWN);
    match kind {
        PartKind::Lever => {
            // 스프링 복귀: 원위치를 살짝 지나쳤다가 돌아온다.
            let decay = (-4.0 * x).exp();
            let swing = (x * std::f32::consts::PI * 1.5).cos();
            (decay * swing).clamp(-0.12, 1.0) * (1.0 - x)
        }
        _ => {
            let y = 1.0 - x;
            y * y
        }
    }
}

pub(crate) fn trigger_pressables(
    mut pressed: MessageReader<PartPressed>,
    mut parts: Query<&mut Pressable>,
) {
    for event in pressed.read() {
        for mut part in &mut parts {
            if part.kind == event.part {
                part.trigger();
            }
        }
    }
}

pub(crate) fn animate_pressables(time: Res<Time>, mut parts: Query<&mut Pressable>) {
    for mut part in &mut parts {
        if !part.is_animating() {
            if part.amount != 0.0 {
                part.amount = 0.0;
            }
            continue;
        }
        part.elapsed = (part.elapsed + time.delta_secs()).min(part.duration);
        let t = if part.duration > 0.0 {
            part.elapsed / part.duration
        } else {
            1.0
        };
        part.amount = if t >= 1.0 {
            0.0
        } else {
            press_curve(part.kind, t)
        };
    }
}
