//! 게임기 플러그인.

use bevy::prelude::*;
use fruit_star_game::GameEvent;

use crate::button::{animate_pressables, trigger_pressables};
use crate::display::{
    DataCounter, DisplayValues, track_data_counter, update_display_values, update_segments,
};
use crate::lamp::{ChanceFlash, Fanfare, LampMode, update_lamp_mode, update_lamps};
use crate::medal::{PayoutCounter, track_payout};
use crate::messages::{
    AppCommand, CameraControl, MachineCommand, MachineEvent, MedalCollision, MedalPaidOut, Notice,
    PartPressed, SessionCommand, StatsSnapshot, UiClick,
};
use crate::state::{AppState, DebugLevel, FailureMessage, Machine, Paused, ViewMode};

/// 한 프레임 안에서의 처리 순서.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MachineSet {
    /// 입력 계층이 명령을 보낸다.
    Input,
    /// 명령을 도메인에 전달하고 시간을 진행한다.
    Simulate,
    /// 도메인 이벤트에 반응해 논리 상태(표시기·램프·눌림)를 갱신한다.
    React,
    /// 논리 상태를 메시·재질·음향에 반영한다.
    Present,
}

/// 게임기 ECS 계층 플러그인. [`Machine`]과 [`crate::Settings`] 리소스는 앱이 넣는다.
pub struct MachinePlugin;

impl Plugin for MachinePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<Paused>()
            .init_resource::<ViewMode>()
            .init_resource::<FailureMessage>()
            .init_resource::<DebugLevel>()
            .init_resource::<crate::state::TouchUi>()
            .init_resource::<crate::state::HudInsets>()
            .init_resource::<DisplayValues>()
            .init_resource::<DataCounter>()
            .init_resource::<PayoutCounter>()
            .init_resource::<LampMode>()
            .init_resource::<Fanfare>()
            .init_resource::<ChanceFlash>()
            .add_message::<MachineCommand>()
            .add_message::<MachineEvent>()
            .add_message::<AppCommand>()
            .add_message::<PartPressed>()
            .add_message::<MedalPaidOut>()
            .add_message::<CameraControl>()
            .add_message::<SessionCommand>()
            .add_message::<Notice>()
            .add_message::<MedalCollision>()
            .add_message::<UiClick>()
            .init_resource::<StatsSnapshot>()
            .configure_sets(
                Update,
                (
                    MachineSet::Input,
                    MachineSet::Simulate,
                    MachineSet::React,
                    MachineSet::Present,
                )
                    .chain(),
            )
            // 명령 처리는 항상 실행한다: 일시정지 중이거나 게임기 앞이 아닐 때 들어온
            // 명령을 그 자리에서 버려, 나중에 뒤늦게 처리되는 일이 없게 한다.
            .add_systems(
                Update,
                (
                    apply_commands,
                    tick_machine.run_if(in_state(AppState::InGame).and_then(not_paused)),
                )
                    .chain()
                    .in_set(MachineSet::Simulate),
            )
            .add_systems(
                Update,
                (
                    track_payout,
                    track_data_counter,
                    update_display_values,
                    update_segments,
                    update_lamp_mode,
                    update_lamps,
                    trigger_pressables,
                    animate_pressables,
                )
                    .chain()
                    .in_set(MachineSet::React)
                    .run_if(in_state(AppState::InGame).and_then(not_paused)),
            )
            .add_systems(OnEnter(AppState::InGame), power_on)
            .add_systems(OnExit(AppState::InGame), power_off);
    }
}

fn not_paused(paused: Res<Paused>) -> bool {
    !paused.0
}

fn power_on(mut machine: ResMut<Machine>, mut events: MessageWriter<MachineEvent>) {
    machine.power_on();
    for event in machine.drain_events() {
        events.write(MachineEvent(event));
    }
}

fn power_off(mut machine: ResMut<Machine>, mut events: MessageWriter<MachineEvent>) {
    machine.power_off();
    for event in machine.drain_events() {
        events.write(MachineEvent(event));
    }
}

/// 조작 명령을 도메인에 전달한다. 도메인이 거부한 입력(중복 정지, 회전 중 베팅 등)은
/// 상태를 바꾸지 않으며, 부품이 눌리는 연출만 일어난다.
fn apply_commands(
    mut commands: MessageReader<MachineCommand>,
    mut machine: ResMut<Machine>,
    view: Res<ViewMode>,
    state: Res<State<AppState>>,
    paused: Res<Paused>,
    settings: Res<crate::state::Settings>,
    mut pressed: MessageWriter<PartPressed>,
) {
    if *state.get() != AppState::InGame || paused.0 {
        commands.clear();
        return;
    }
    for command in commands.read() {
        if !view.playable() {
            continue;
        }
        let result = match *command {
            MachineCommand::InsertMedal => {
                machine.insert_medals(settings.config.application.game.medals_per_insert)
            }
            MachineCommand::Bet => machine.press_bet(),
            MachineCommand::BetOne => machine.press_bet_one(),
            MachineCommand::Lever => machine.pull_lever(),
            MachineCommand::Stop(reel) => machine.press_stop(reel),
        };
        if let Err(error) = &result {
            debug!("입력 거부: {command:?}: {error}");
        }
        pressed.write(PartPressed {
            part: command.part(),
            accepted: result.is_ok(),
        });
    }
}

/// 도메인의 시간을 진행하고 발생한 이벤트를 메시지로 내보낸다.
fn tick_machine(
    time: Res<Time>,
    mut machine: ResMut<Machine>,
    mut events: MessageWriter<MachineEvent>,
) {
    machine.tick(time.delta_secs_f64());
    for event in machine.drain_events() {
        if let GameEvent::GameCompleted(outcome) = &event {
            debug!(
                "게임 {} 완료: 추첨 {:?}, 지급 {}",
                outcome.game_id, outcome.flag, outcome.payout
            );
        }
        events.write(MachineEvent(event));
    }
}
