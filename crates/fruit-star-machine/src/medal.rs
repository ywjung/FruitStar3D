//! 메달 지급 진행 (한 장씩 배출).

use bevy::prelude::*;
use fruit_star_game::GameEvent;

use crate::messages::{MachineEvent, MedalPaidOut};
use crate::state::Machine;

/// 받침으로 떨어지는 메달. 물리 계층이 만들고 렌더링 계층이 모양을 입힌다.
#[derive(Component, Debug, Clone, Copy)]
pub struct Medal {
    /// 만들어진(또는 재사용된) 순서. 작을수록 오래된 메달이다.
    pub serial: u64,
}

/// 지급 중인 메달의 진행 상태. 도메인 원장은 이미 정산이 끝난 상태이며,
/// 이 카운터는 표시기와 메달 연출의 진행만 관리한다.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PayoutCounter {
    /// 아직 배출되지 않은 메달 수.
    pub pending: u32,
    /// 이번 게임에서 지금까지 배출된 메달 수 (PAYOUT 표시기 값).
    pub paid: u32,
    timer: f32,
}

pub(crate) fn track_payout(
    time: Res<Time>,
    machine: Res<Machine>,
    mut events: MessageReader<MachineEvent>,
    mut counter: ResMut<PayoutCounter>,
    mut paid_out: MessageWriter<MedalPaidOut>,
) {
    for MachineEvent(event) in events.read() {
        match event {
            GameEvent::LeverPulled { .. } => {
                // 다음 게임이 시작되면 남은 연출은 즉시 마무리한다.
                counter.pending = 0;
                counter.paid = 0;
                counter.timer = 0.0;
            }
            GameEvent::MedalDispensed(count) => {
                counter.pending += *count;
                counter.paid = 0;
                counter.timer = 0.0;
            }
            _ => {}
        }
    }

    if counter.pending == 0 {
        return;
    }
    let interval = machine.profile().flow.payout_interval_s as f32;
    counter.timer += time.delta_secs();
    // 프레임이 길어져도 메달 수가 어긋나지 않도록 밀린 만큼 한꺼번에 처리한다.
    while counter.pending > 0 && (interval <= 0.0 || counter.timer >= interval) {
        counter.timer -= interval.max(0.0);
        counter.pending -= 1;
        counter.paid += 1;
        paid_out.write(MedalPaidOut {
            remaining: counter.pending,
        });
        if interval <= 0.0 && counter.pending == 0 {
            break;
        }
    }
}
