//! FRUIT STAR 3D 음향 계층.
//!
//! 게임기 이벤트를 구독해 기계음을 낸다. 릴·버튼·메달 소리는 부품 위치에서 나는
//! 공간 음향이고, 반복음(릴 회전, 환경음)은 릴이 멈추거나 화면이 바뀌면 끝낸다.
//! 효과음과 배경음은 음량이 독립적이다.

use std::time::Duration;

use bevy::audio::{AudioSinkPlayback, PlaybackMode, SpatialListener, Volume};
use bevy::prelude::*;
use fruit_star_game::GameEvent;
use fruit_star_machine::layout::{
    BET_BUTTON_POS, CHUTE_POS, LEVER_PIVOT, MEDAL_SLOT_POS, reel_center, stop_button_pos,
};
use fruit_star_machine::{
    AppState, Machine, MachineEvent, MachineSet, MainCamera, MedalCollision, MedalPaidOut,
    PartKind, PartPressed, Paused, Settings, UiClick,
};

/// 음향 플러그인.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpinningReels>()
            .add_systems(Startup, load_sounds)
            .add_systems(
                Update,
                (
                    attach_listener,
                    machine_sounds,
                    part_sounds,
                    medal_sounds,
                    ui_sounds,
                    play_delayed,
                    sync_volume,
                )
                    .in_set(MachineSet::Present)
                    .run_if(resource_exists::<Sounds>),
            )
            .add_systems(OnEnter(AppState::InGame), start_ambient)
            .add_systems(OnExit(AppState::InGame), stop_loops);
    }
}

/// 효과음 핸들.
#[derive(Resource, Debug, Clone)]
pub struct Sounds {
    power_on: Handle<AudioSource>,
    ambient: Handle<AudioSource>,
    lever_pull: Handle<AudioSource>,
    lever_return: Handle<AudioSource>,
    reel_start: Handle<AudioSource>,
    reel_loop: Handle<AudioSource>,
    reel_stop: Handle<AudioSource>,
    button_press: Handle<AudioSource>,
    button_release: Handle<AudioSource>,
    medal_insert: Handle<AudioSource>,
    medal_drop: Handle<AudioSource>,
    medal_collision: Handle<AudioSource>,
    medal_tray: Handle<AudioSource>,
    bonus_start: Handle<AudioSource>,
    bonus_end: Handle<AudioSource>,
    menu_select: Handle<AudioSource>,
    win_small: Handle<AudioSource>,
    win_big: Handle<AudioSource>,
    replay: Handle<AudioSource>,
    bet: Handle<AudioSource>,
    bonus_loop: Handle<AudioSource>,
    chance_lamp: Handle<AudioSource>,
}

fn load_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    let load = |name: &str| assets.load(format!("audio/{name}.wav"));
    commands.insert_resource(Sounds {
        power_on: load("power_on"),
        ambient: load("ambient"),
        lever_pull: load("lever_pull"),
        lever_return: load("lever_return"),
        reel_start: load("reel_start"),
        reel_loop: load("reel_loop"),
        reel_stop: load("reel_stop"),
        button_press: load("button_press"),
        button_release: load("button_release"),
        medal_insert: load("medal_insert"),
        medal_drop: load("medal_drop"),
        medal_collision: load("medal_collision"),
        medal_tray: load("medal_tray"),
        bonus_start: load("bonus_start"),
        bonus_end: load("bonus_end"),
        menu_select: load("menu_select"),
        win_small: load("win_small"),
        win_big: load("win_big"),
        replay: load("replay"),
        bet: load("bet"),
        bonus_loop: load("bonus_loop"),
        chance_lamp: load("chance_lamp"),
    });
}

/// 반복 재생 중인 소리의 종류.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
enum LoopSound {
    Ambient,
    /// 릴 회전음. 세 릴이 함께 쓰는 하나의 소리이며, 도는 릴 수에 따라 음량이 달라진다.
    Reels,
    /// 보너스 게임 중의 배경 음악.
    Bonus,
}

/// 믹스 배율: 원본 파일은 모두 같은 피크로 정규화되어 있어 여기서 상대 음량을 정한다.
const REEL_LOOP_GAIN: f32 = 0.24;

/// 지금 돌고 있는 릴 수. 회전음의 음량과 종료 시점을 정한다.
#[derive(Resource, Debug, Clone, Copy, Default)]
struct SpinningReels(u8);

const AMBIENT_GAIN: f32 = 1.0;
const BONUS_MUSIC_GAIN: f32 = 1.6;

/// 이 지급 수 이상이면 큰 당첨음을 낸다.
const BIG_WIN_PAYOUT: u32 = 10;

/// 온음씩 올라가는 음높이 배율 (도·레·미). 걸린 메달 수를 알리는 베팅음에 쓴다.
const STEP_PITCH: [f32; 3] = [1.0, 1.122_462, 1.259_921];

/// 일정 시간 뒤에 낼 소리 (버튼·레버가 돌아오는 소리).
#[derive(Component, Debug)]
struct DelayedSound {
    timer: Timer,
    source: Handle<AudioSource>,
    position: Vec3,
    gain: f32,
}

fn attach_listener(
    mut commands: Commands,
    cameras: Query<Entity, (With<MainCamera>, Without<SpatialListener>)>,
) {
    for camera in &cameras {
        // 두 귀 사이 간격 20cm
        commands.entity(camera).insert(SpatialListener::new(0.2));
    }
}

/// 한 번 재생하고 사라지는 소리를 낸다. `position`이 있으면 그 위치에서 나는 공간 음향이다.
fn play(
    commands: &mut Commands,
    settings: &Settings,
    source: &Handle<AudioSource>,
    position: Option<Vec3>,
    gain: f32,
    speed: f32,
) {
    let audio = &settings.config.audio;
    let volume = audio.effective_effects() * gain;
    if volume <= 0.0 {
        return;
    }
    let spatial = position.is_some() && audio.spatial;
    commands.spawn((
        Name::new("Sound"),
        AudioPlayer::new(source.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(volume),
            speed,
            spatial,
            ..default()
        },
        Transform::from_translation(position.unwrap_or(Vec3::ZERO)),
    ));
}

fn start_ambient(mut commands: Commands, sounds: Option<Res<Sounds>>, settings: Res<Settings>) {
    let Some(sounds) = sounds else {
        return;
    };
    commands.spawn((
        Name::new("AmbientLoop"),
        LoopSound::Ambient,
        AudioPlayer::new(sounds.ambient.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(settings.config.audio.effective_ambient() * AMBIENT_GAIN),
            ..default()
        },
    ));
    play(&mut commands, &settings, &sounds.power_on, None, 0.6, 1.0);
}

/// 화면이 바뀌면 반복음을 모두 끝낸다.
fn stop_loops(
    mut spinning: ResMut<SpinningReels>,
    mut commands: Commands,
    loops: Query<Entity, With<LoopSound>>,
    delayed: Query<Entity, With<DelayedSound>>,
) {
    spinning.0 = 0;
    for entity in loops.iter().chain(delayed.iter()) {
        commands.entity(entity).despawn();
    }
}

fn machine_sounds(
    mut commands: Commands,
    mut events: MessageReader<MachineEvent>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    loops: Query<(Entity, &LoopSound)>,
    mut spinning: ResMut<SpinningReels>,
) {
    for MachineEvent(event) in events.read() {
        match event {
            GameEvent::LeverPulled { .. } => {
                spinning.0 = 0;
            }
            GameEvent::BetPressed { total, replay, .. } => {
                // 리플레이의 자동 베팅은 리플레이음이 대신한다.
                if !replay {
                    // 걸린 메달 수만큼 음이 올라간다.
                    let step = (*total as usize).clamp(1, STEP_PITCH.len()) - 1;
                    play(
                        &mut commands,
                        &settings,
                        &sounds.bet,
                        None,
                        0.3,
                        STEP_PITCH[step],
                    );
                }
            }
            GameEvent::ReelStarted(_) => {
                spinning.0 = spinning.0.saturating_add(1);
                // 시작음과 회전음은 세 릴이 함께 쓴다. 릴마다 따로 틀면 같은 소리가 겹쳐
                // 음량이 커지고, 음높이를 달리하면 서로 맥놀이를 일으킨다.
                if !loops.iter().any(|(_, sound)| *sound == LoopSound::Reels) && spinning.0 == 1 {
                    let centre = reel_center(1);
                    play(
                        &mut commands,
                        &settings,
                        &sounds.reel_start,
                        Some(centre),
                        0.35,
                        1.0,
                    );
                    let audio = &settings.config.audio;
                    commands.spawn((
                        Name::new("ReelLoop"),
                        LoopSound::Reels,
                        AudioPlayer::new(sounds.reel_loop.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Loop,
                            volume: Volume::Linear(0.0),
                            spatial: audio.spatial,
                            ..default()
                        },
                        Transform::from_translation(centre),
                    ));
                }
            }
            GameEvent::ReelStopped { reel, .. } => {
                spinning.0 = spinning.0.saturating_sub(1);
                if spinning.0 == 0 {
                    for (entity, sound) in &loops {
                        if *sound == LoopSound::Reels {
                            commands.entity(entity).despawn();
                        }
                    }
                }
                // 브레이크의 작은 개체 차이는 릴 위치에 고정된다.
                play(
                    &mut commands,
                    &settings,
                    &sounds.reel_stop,
                    Some(reel_center(*reel)),
                    0.6,
                    [0.977, 1.0, 1.026][*reel],
                );
            }
            GameEvent::MedalInserted { .. } => {
                play(
                    &mut commands,
                    &settings,
                    &sounds.medal_insert,
                    Some(MEDAL_SLOT_POS),
                    0.5,
                    1.0,
                );
            }
            GameEvent::GameCompleted(outcome) => {
                if outcome.bonus_started.is_some() {
                    // 보너스 시작음이 따로 난다.
                } else if outcome.payout >= BIG_WIN_PAYOUT {
                    play(&mut commands, &settings, &sounds.win_big, None, 0.5, 1.0);
                } else if outcome.payout > 0 {
                    play(&mut commands, &settings, &sounds.win_small, None, 0.4, 1.0);
                } else if outcome.replay_awarded {
                    play(&mut commands, &settings, &sounds.replay, None, 0.35, 1.0);
                }
            }
            GameEvent::BonusStarted(_) => {
                play(
                    &mut commands,
                    &settings,
                    &sounds.bonus_start,
                    None,
                    0.7,
                    1.0,
                );
                // 보너스 게임 동안 배경 음악을 깐다.
                if !loops.iter().any(|(_, sound)| *sound == LoopSound::Bonus) {
                    commands.spawn((
                        Name::new("BonusMusic"),
                        LoopSound::Bonus,
                        AudioPlayer::new(sounds.bonus_loop.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Loop,
                            volume: Volume::Linear(
                                settings.config.audio.effective_ambient() * BONUS_MUSIC_GAIN,
                            ),
                            ..default()
                        },
                    ));
                }
            }
            GameEvent::BonusCompleted { .. } => {
                for (entity, sound) in &loops {
                    if *sound == LoopSound::Bonus {
                        commands.entity(entity).despawn();
                    }
                }
                play(&mut commands, &settings, &sounds.bonus_end, None, 0.6, 1.0);
            }
            GameEvent::ChanceLampChanged(true) => {
                play(
                    &mut commands,
                    &settings,
                    &sounds.chance_lamp,
                    None,
                    0.7,
                    1.0,
                );
            }
            _ => {}
        }
    }
}

fn part_sounds(
    mut commands: Commands,
    mut pressed: MessageReader<PartPressed>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
) {
    let animation = &settings.config.machine.animation;
    for event in pressed.read() {
        let (position, press, release, return_ms) = match event.part {
            PartKind::Lever => (
                LEVER_PIVOT,
                &sounds.lever_pull,
                &sounds.lever_return,
                animation.lever_ms * 2 / 5,
            ),
            PartKind::BetOne => (
                fruit_star_machine::layout::BET_ONE_BUTTON_POS,
                &sounds.button_press,
                &sounds.button_release,
                animation.bet_button_ms / 2,
            ),
            PartKind::Bet => (
                BET_BUTTON_POS,
                &sounds.button_press,
                &sounds.button_release,
                animation.bet_button_ms / 2,
            ),
            PartKind::Stop(reel) => (
                stop_button_pos(reel),
                &sounds.button_press,
                &sounds.button_release,
                animation.stop_button_ms / 2,
            ),
            // 투입구는 메달 투입음이 따로 난다.
            PartKind::MedalSlot => continue,
        };
        let (press_gain, release_gain) = match event.part {
            PartKind::Lever => (0.6, 0.4),
            _ => (0.5, 0.3),
        };
        play(
            &mut commands,
            &settings,
            press,
            Some(position),
            press_gain,
            1.0,
        );
        commands.spawn(DelayedSound {
            timer: Timer::new(
                Duration::from_millis(u64::from(return_ms.max(1))),
                TimerMode::Once,
            ),
            source: release.clone(),
            position,
            gain: release_gain,
        });
    }
}

fn play_delayed(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<Settings>,
    mut delayed: Query<(Entity, &mut DelayedSound)>,
) {
    for (entity, mut sound) in &mut delayed {
        if sound.timer.tick(time.delta()).is_finished() {
            play(
                &mut commands,
                &settings,
                &sound.source,
                Some(sound.position),
                sound.gain,
                1.0,
            );
            commands.entity(entity).despawn();
        }
    }
}

fn medal_sounds(
    mut commands: Commands,
    mut paid: MessageReader<MedalPaidOut>,
    mut collisions: MessageReader<MedalCollision>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
    mut budget: Local<f32>,
    mut pocket: Local<usize>,
    time: Res<Time>,
) {
    // 한 프레임에 몰려 배출된 메달은 같은 위치의 타격 하나로 들려준다.
    let count = paid.read().count();
    if count > 0 {
        const POCKET_SPEEDS: [f32; 7] = [1.0, 0.976, 1.012, 0.991, 1.027, 0.983, 1.006];
        let speed = POCKET_SPEEDS[*pocket];
        *pocket = (*pocket + count) % POCKET_SPEEDS.len();
        play(
            &mut commands,
            &settings,
            &sounds.medal_drop,
            Some(CHUTE_POS),
            0.37,
            speed,
        );
    }

    // 같은 프레임에 여러 장이 겹쳐도 재질별로 가장 큰 접촉을 들려준다.
    *budget = (*budget + time.delta_secs() * 18.0).min(3.0);
    let mut strongest: [Option<&MedalCollision>; 2] = [None, None];
    for hit in collisions.read() {
        let candidate = &mut strongest[usize::from(hit.other_medal)];
        if candidate.is_none_or(|previous| hit.strength > previous.strength) {
            *candidate = Some(hit);
        }
    }
    strongest.sort_by(|a, b| {
        let level = |hit: &Option<&MedalCollision>| hit.map_or(0.0, |h| h.strength);
        level(b).total_cmp(&level(a))
    });
    for hit in strongest.into_iter().flatten() {
        if *budget < 1.0 {
            break;
        }
        *budget -= 1.0;
        let strength = hit.strength.clamp(0.0, 1.0);
        let (sound, gain) = if hit.other_medal {
            (&sounds.medal_collision, 0.38)
        } else {
            (&sounds.medal_tray, 0.54)
        };
        let speed = 0.98 + strength * 0.035 + 0.018 * (hit.position.x * 93.0).sin();
        play(
            &mut commands,
            &settings,
            sound,
            Some(hit.position),
            gain * strength.sqrt(),
            speed,
        );
    }
}

fn ui_sounds(
    mut commands: Commands,
    mut clicks: MessageReader<UiClick>,
    sounds: Res<Sounds>,
    settings: Res<Settings>,
) {
    if clicks.read().next().is_some() {
        clicks.clear();
        play(
            &mut commands,
            &settings,
            &sounds.menu_select,
            None,
            0.3,
            1.0,
        );
    }
}

/// 음량 설정·음소거·일시정지를 재생 중인 반복음에 반영한다.
fn sync_volume(
    settings: Res<Settings>,
    paused: Res<Paused>,
    machine: Res<Machine>,
    mut plain: Query<(&LoopSound, &mut AudioSink)>,
    mut spatial: Query<(&LoopSound, &mut SpatialAudioSink)>,
) {
    let audio = &settings.config.audio;
    // 가속·감속과 한 개씩 정지하는 소리를 렌더링과 같은 릴 속도에서 구한다.
    let speeds = machine
        .reels()
        .each_ref()
        .map(|reel| reel.angular_velocity().abs());
    let rated = (settings.config.machine.reel.spin_rpm as f32) * std::f32::consts::TAU / 60.0;
    let reel_level = (speeds.iter().map(|v| (v / rated).powi(2)).sum::<f32>() / 3.0)
        .sqrt()
        .min(1.0);
    let nominal = 80.0 * std::f32::consts::TAU / 60.0;
    let reel_speed = (speeds.into_iter().fold(0.0, f32::max) / nominal).max(0.25);
    let mix = |sound: &LoopSound| match sound {
        LoopSound::Ambient => (audio.effective_ambient() * AMBIENT_GAIN, 1.0),
        LoopSound::Reels => (
            audio.effective_effects() * REEL_LOOP_GAIN * reel_level,
            reel_speed,
        ),
        LoopSound::Bonus => (audio.effective_ambient() * BONUS_MUSIC_GAIN, 1.0),
    };
    for (sound, mut sink) in &mut plain {
        apply_sink(&mut *sink, mix(sound), paused.0);
    }
    for (sound, mut sink) in &mut spatial {
        apply_sink(&mut *sink, mix(sound), paused.0);
    }
}

fn apply_sink(sink: &mut impl AudioSinkPlayback, (volume, speed): (f32, f32), paused: bool) {
    if (sink.speed() - speed).abs() > 0.002 {
        sink.set_speed(speed);
    }
    if (sink.volume().to_linear() - volume).abs() > 0.001 {
        sink.set_volume(Volume::Linear(volume));
    }
    if paused != sink.is_paused() {
        if paused {
            sink.pause();
        } else {
            sink.play();
        }
    }
}
