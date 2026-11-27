#![allow(clippy::unwrap_used, clippy::expect_used)]

//! 게임기 ECS 계층의 헤드리스 통합 테스트 (창·GPU 없이 실행).

use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use fruit_star_config::ConfigSet;
use fruit_star_core::{RandomSource, ReelTiming, SplitMix64};
use fruit_star_game::{Flag, GameController, GameEvent, MachineProfile, Phase, PlayMode, Role};
use fruit_star_machine::{
    AppState, DataCounter, DisplayKind, DisplayValues, Lamp, LampKind, Machine, MachineCommand,
    MachineEvent, MachinePlugin, MedalPaidOut, PartKind, PartPressed, Paused, Pressable, Settings,
    SevenSegment, StatusLamp, ViewMode,
};

/// 추첨에서 항상 같은 값을 내는 난수 공급원.
struct Fixed(u32);

impl RandomSource for Fixed {
    fn next_u32(&mut self) -> u32 {
        self.0 << 16
    }
}

/// 수집한 메시지.
#[derive(Resource, Default)]
struct Collected {
    events: Vec<GameEvent>,
    pressed: Vec<PartPressed>,
    medals: u32,
}

fn collect(
    mut events: MessageReader<MachineEvent>,
    mut pressed: MessageReader<PartPressed>,
    mut medals: MessageReader<MedalPaidOut>,
    mut collected: ResMut<Collected>,
) {
    for MachineEvent(event) in events.read() {
        collected.events.push(event.clone());
    }
    collected.pressed.extend(pressed.read().copied());
    collected.medals += medals.read().count() as u32;
}

fn app_with(rng: Box<dyn RandomSource + Send + Sync>, credit: u32) -> App {
    let controller = GameController::new(
        MachineProfile::fruit_classic(),
        ReelTiming::default(),
        PlayMode::Classic,
        credit,
        9_999,
        1,
        rng,
    )
    .unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, MachinePlugin))
        .insert_resource(Machine(controller))
        .insert_resource(Settings {
            config: ConfigSet::default(),
            root: std::path::PathBuf::from("."),
        })
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            10,
        )))
        .init_resource::<Collected>()
        .add_systems(Update, collect.after(fruit_star_machine::MachineSet::React));
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    app.update();
    app.update();
    app
}

fn send(app: &mut App, command: MachineCommand) {
    app.world_mut().write_message(command);
    app.update();
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn count(app: &App, pred: impl Fn(&GameEvent) -> bool) -> usize {
    app.world()
        .resource::<Collected>()
        .events
        .iter()
        .filter(|e| pred(e))
        .count()
}

fn phase(app: &App) -> Phase {
    app.world().resource::<Machine>().phase()
}

#[test]
fn entering_the_game_powers_the_machine_on_once() {
    let app = app_with(Box::new(SplitMix64::new(1)), 50);
    assert_eq!(count(&app, |e| matches!(e, GameEvent::PowerOn)), 1);
}

#[test]
fn lever_input_produces_exactly_one_event_and_duplicates_are_ignored() {
    let mut app = app_with(Box::new(Fixed(65_535)), 50);
    // 베팅 없이 레버: 부품은 눌리지만 게임은 시작되지 않는다
    send(&mut app, MachineCommand::Lever);
    assert_eq!(
        count(&app, |e| matches!(e, GameEvent::LeverPulled { .. })),
        0
    );
    let pressed = app.world().resource::<Collected>().pressed.clone();
    assert_eq!(
        pressed,
        vec![PartPressed {
            part: PartKind::Lever,
            accepted: false
        }]
    );

    send(&mut app, MachineCommand::Bet);
    // 같은 프레임에 레버 입력이 세 번 들어와도 게임은 한 번만 시작된다
    for _ in 0..3 {
        app.world_mut().write_message(MachineCommand::Lever);
    }
    app.update();
    send(&mut app, MachineCommand::Lever);
    assert_eq!(
        count(&app, |e| matches!(e, GameEvent::LeverPulled { .. })),
        1
    );
    assert_eq!(count(&app, |e| matches!(e, GameEvent::ReelStarted(_))), 3);
    assert_eq!(
        count(&app, |e| matches!(e, GameEvent::BetPressed { .. })),
        1
    );
    assert_eq!(app.world().resource::<Machine>().credit(), 47);

    // 정속 도달 뒤 정지: 같은 릴을 여러 번 눌러도 한 번만 처리된다
    run(&mut app, 50);
    for _ in 0..4 {
        send(&mut app, MachineCommand::Stop(0));
    }
    send(&mut app, MachineCommand::Stop(7)); // 없는 릴
    assert_eq!(count(&app, |e| matches!(e, GameEvent::StopPressed(0))), 1);
    assert_eq!(count(&app, |e| matches!(e, GameEvent::StopPressed(_))), 1);
    send(&mut app, MachineCommand::Stop(1));
    send(&mut app, MachineCommand::Stop(2));
    run(&mut app, 200);
    assert_eq!(phase(&app), Phase::Ready);
    assert_eq!(
        count(&app, |e| matches!(e, GameEvent::ReelStopped { .. })),
        3
    );
    assert_eq!(count(&app, |e| matches!(e, GameEvent::GameCompleted(_))), 1);
}

#[test]
fn payout_is_dispensed_one_medal_at_a_time_and_displays_follow() {
    let profile = MachineProfile::fruit_classic();
    let mut start = 0;
    for (flag, weight) in profile.lottery.entries() {
        if *flag == Flag::Small(Role::Grape) {
            break;
        }
        start += weight;
    }
    let mut app = app_with(Box::new(Fixed(start)), 50);
    send(&mut app, MachineCommand::Bet);
    send(&mut app, MachineCommand::Lever);
    run(&mut app, 50);
    for reel in 0..3 {
        send(&mut app, MachineCommand::Stop(reel));
    }
    // 정산 직후: 원장은 이미 +8이지만 표시기는 지급 연출에 맞춰 한 장씩 오른다
    let mut saw_partial = false;
    for _ in 0..400 {
        app.update();
        let machine_credit = app.world().resource::<Machine>().credit();
        let display = *app.world().resource::<DisplayValues>();
        let medals = app.world().resource::<Collected>().medals;
        if machine_credit == 55 {
            assert_eq!(display.credit + (8 - display.payout), 55);
            assert_eq!(display.payout, medals);
            if display.payout > 0 && display.payout < 8 {
                saw_partial = true;
            }
        }
        if phase(&app) == Phase::Ready && medals == 8 {
            break;
        }
    }
    assert!(saw_partial, "지급이 한 장씩 진행되어야 한다");
    let collected = app.world().resource::<Collected>();
    assert_eq!(collected.medals, 8, "메달 배출 수는 지급 수와 같아야 한다");
    assert_eq!(
        collected
            .events
            .iter()
            .filter(|e| matches!(e, GameEvent::MedalDispensed(8)))
            .count(),
        1
    );
    let display = *app.world().resource::<DisplayValues>();
    assert_eq!(display.credit, 55);
    assert_eq!(display.payout, 8);
    assert_eq!(display.count, None);
}

#[test]
fn segments_lamps_and_pressables_react() {
    let mut app = app_with(Box::new(Fixed(65_535)), 50);
    let world = app.world_mut();
    for digit in 0..4 {
        for segment in 0..7 {
            world.spawn(SevenSegment {
                display: DisplayKind::Credit,
                digit,
                segment,
                lit: false,
            });
        }
    }
    let stop_lamp = world.spawn(Lamp::new(LampKind::StopButton(0))).id();
    let bet_lamp = world.spawn(Lamp::new(LampKind::BetButton)).id();
    let lever = world.spawn(Pressable::new(PartKind::Lever, 250)).id();
    app.update();

    // CREDIT 50 → 십의 자리 5, 일의 자리 0
    let lit = |app: &mut App, digit: u8| -> u8 {
        let mut mask = 0;
        let mut query = app.world_mut().query::<&SevenSegment>();
        for segment in query.iter(app.world()) {
            if segment.digit == digit && segment.lit {
                mask |= 1 << segment.segment;
            }
        }
        mask
    };
    assert_eq!(lit(&mut app, 1), 0b1101101);
    assert_eq!(lit(&mut app, 0), 0b0111111);
    assert_eq!(lit(&mut app, 2), 0);

    // 대기 중: 베팅 램프는 켜지고 정지 버튼 램프는 꺼져 있다
    assert!(app.world().get::<Lamp>(bet_lamp).unwrap().brightness > 0.3);
    assert!(app.world().get::<Lamp>(stop_lamp).unwrap().brightness < 0.2);

    send(&mut app, MachineCommand::Bet);
    send(&mut app, MachineCommand::Lever);
    assert!(app.world().get::<Pressable>(lever).unwrap().is_animating());
    run(&mut app, 8);
    assert!(app.world().get::<Pressable>(lever).unwrap().amount > 0.3);
    run(&mut app, 50);
    // 정속 회전 중: 정지 버튼 램프가 켜진다
    assert!(app.world().get::<Lamp>(stop_lamp).unwrap().brightness > 0.9);
    // 레버는 원위치로 돌아왔다
    let lever_state = app.world().get::<Pressable>(lever).unwrap();
    assert!(!lever_state.is_animating());
    assert_eq!(lever_state.amount, 0.0);
    assert_eq!(lit(&mut app, 1), 0b1100110, "CREDIT 47의 십의 자리 4");
}

#[test]
fn pause_and_gallery_block_machine_input() {
    let mut app = app_with(Box::new(Fixed(65_535)), 50);
    app.world_mut().resource_mut::<Paused>().0 = true;
    send(&mut app, MachineCommand::Bet);
    assert_eq!(app.world().resource::<Machine>().credit(), 50);
    app.world_mut().resource_mut::<Paused>().0 = false;
    app.update();
    // 일시정지 중에 들어온 입력이 풀린 뒤에 뒤늦게 처리되어서도 안 된다
    run(&mut app, 3);
    assert_eq!(app.world().resource::<Machine>().credit(), 50);

    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::Gallery;
    send(&mut app, MachineCommand::Bet);
    assert_eq!(app.world().resource::<Machine>().credit(), 50);
    assert!(app.world().resource::<Collected>().pressed.is_empty());

    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::Play;
    send(&mut app, MachineCommand::Bet);
    assert_eq!(app.world().resource::<Machine>().credit(), 47);
}

#[test]
fn winning_line_lamps_light_only_for_the_winning_line_until_the_next_game() {
    let profile = MachineProfile::fruit_classic();
    let mut start = 0;
    for (flag, weight) in profile.lottery.entries() {
        if *flag == Flag::Small(Role::Grape) {
            break;
        }
        start += weight;
    }
    let mut app = app_with(Box::new(Fixed(start)), 50);
    let lamps: Vec<Entity> = (0..5u8)
        .map(|line| {
            app.world_mut()
                .spawn(Lamp::new(LampKind::PayLine(line)))
                .id()
        })
        .collect();
    let brightness = |app: &App| -> Vec<f32> {
        lamps
            .iter()
            .map(|entity| app.world().get::<Lamp>(*entity).unwrap().brightness)
            .collect()
    };

    send(&mut app, MachineCommand::Bet);
    send(&mut app, MachineCommand::Lever);
    run(&mut app, 50);
    // 릴이 도는 동안에는 어떤 라인도 켜지지 않는다
    assert!(brightness(&app).iter().all(|b| *b == 0.0));
    for reel in 0..3 {
        send(&mut app, MachineCommand::Stop(reel));
    }
    run(&mut app, 400);
    assert_eq!(phase(&app), Phase::Ready);

    let outcome = app
        .world()
        .resource::<Machine>()
        .last_outcome()
        .cloned()
        .unwrap();
    assert!(!outcome.wins.is_empty());
    let lit = brightness(&app);
    for line in 0..5u8 {
        let won = outcome.wins.iter().any(|win| win.line == line);
        assert_eq!(lit[line as usize] > 0.5, won, "라인 {line}: {lit:?}");
    }

    // 다음 게임의 레버를 당기면 꺼진다
    send(&mut app, MachineCommand::Bet);
    send(&mut app, MachineCommand::Lever);
    app.update();
    assert!(brightness(&app).iter().all(|b| *b == 0.0));
}

#[test]
fn line_lamps_follow_the_bet_and_status_lamps_guide_the_player() {
    let mut app = app_with(Box::new(Fixed(65_535)), 3);
    let lines: Vec<Entity> = (0..5u8)
        .map(|line| {
            app.world_mut()
                .spawn(Lamp::new(LampKind::LineLamp(line)))
                .id()
        })
        .collect();
    let start = app
        .world_mut()
        .spawn(Lamp::new(LampKind::Status(StatusLamp::Start)))
        .id();
    let insert = app
        .world_mut()
        .spawn(Lamp::new(LampKind::Status(StatusLamp::Insert)))
        .id();
    let one_bet = app
        .world_mut()
        .spawn(Lamp::new(LampKind::BetOneButton))
        .id();
    let lit = |app: &App, entity: Entity| app.world().get::<Lamp>(entity).unwrap().brightness > 0.3;
    let lit_lines = |app: &App| -> Vec<bool> { lines.iter().map(|e| lit(app, *e)).collect() };

    app.update();
    // 아무것도 걸지 않았다: 라인 등은 모두 꺼져 있고 1 BET 버튼이 눌러 달라고 빛난다
    assert_eq!(lit_lines(&app), [false; 5]);
    assert!(!lit(&app, start));
    assert!(lit(&app, one_bet));

    // 1메달: 가운데 줄만
    send(&mut app, MachineCommand::BetOne);
    assert_eq!(lit_lines(&app), [true, false, false, false, false]);
    assert!(lit(&app, start), "베팅하면 START 등이 켜진다");
    // 2메달: 가로 세 줄
    send(&mut app, MachineCommand::BetOne);
    assert_eq!(lit_lines(&app), [true, true, true, false, false]);
    // 3메달: 다섯 줄 모두
    send(&mut app, MachineCommand::BetOne);
    assert_eq!(lit_lines(&app), [true; 5]);
    assert_eq!(app.world().resource::<Machine>().credit(), 0);
    // 더 걸 수 없다
    send(&mut app, MachineCommand::BetOne);
    assert_eq!(app.world().resource::<Machine>().bet_amount(), 3);

    // 게임을 끝낸다 (꽝). 메달이 0이므로 INSERT MEDAL 등이 켜진다
    send(&mut app, MachineCommand::Lever);
    assert!(!lit(&app, start));
    // 회전 중에도 이번 게임의 유효 라인은 켜져 있다
    assert_eq!(lit_lines(&app), [true; 5]);
    run(&mut app, 50);
    for reel in 0..3 {
        send(&mut app, MachineCommand::Stop(reel));
    }
    run(&mut app, 200);
    assert_eq!(phase(&app), Phase::Ready);
    assert_eq!(lit_lines(&app), [false; 5]);
    assert!(lit(&app, insert), "메달이 없으면 INSERT MEDAL 등이 켜진다");
    send(&mut app, MachineCommand::InsertMedal);
    assert!(!lit(&app, insert));
}

#[test]
fn data_counter_counts_games_and_resets_after_a_bonus() {
    let profile = MachineProfile::fruit_classic();
    let mut start = 0;
    for (flag, weight) in profile.lottery.entries() {
        if *flag == Flag::Bonus(fruit_star_game::BonusKind::Regular) {
            break;
        }
        start += weight;
    }
    // 첫 게임은 꽝, 두 번째 게임에서 REGULAR 보너스 당첨
    struct Sequence(Vec<u32>, usize);
    impl RandomSource for Sequence {
        fn next_u32(&mut self) -> u32 {
            let value = self.0.get(self.1).copied().unwrap_or(65_535);
            self.1 += 1;
            (value << 16) | 0xFFFF
        }
    }
    let mut app = app_with(Box::new(Sequence(vec![65_535, start], 0)), 50);
    let play = |app: &mut App, aim_bar: bool| {
        send(app, MachineCommand::Bet);
        send(app, MachineCommand::Lever);
        run(app, 50);
        for (reel, strip) in profile.strips.iter().enumerate() {
            if aim_bar {
                // BAR가 가운데 줄에 올 때까지 기다렸다가 누른다
                let target = strip
                    .iter()
                    .position(|s| *s == fruit_star_core::Symbol::Bar)
                    .unwrap();
                for _ in 0..400 {
                    if app.world().resource::<Machine>().reels()[reel].next_index() == target {
                        break;
                    }
                    app.update();
                }
            }
            send(app, MachineCommand::Stop(reel));
        }
        run(app, 300);
    };

    play(&mut app, false);
    let counter = *app.world().resource::<DataCounter>();
    assert_eq!((counter.total_games, counter.since_bonus), (1, 1));
    assert_eq!((counter.big, counter.regular), (0, 0));

    play(&mut app, true);
    let counter = *app.world().resource::<DataCounter>();
    assert!(
        app.world().resource::<Machine>().bonus().is_some(),
        "BAR를 맞춰 보너스가 시작되어야 한다"
    );
    assert_eq!(counter.total_games, 2);
    assert_eq!(counter.regular, 1);
    assert_eq!(
        counter.since_bonus, 0,
        "보너스가 시작되면 게임 수가 0으로 돌아간다"
    );

    // 보너스 게임은 총 게임 수에만 더해진다
    play(&mut app, false);
    let counter = *app.world().resource::<DataCounter>();
    assert_eq!(counter.total_games, 3);
    assert_eq!(counter.since_bonus, 0);
}
