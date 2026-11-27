//! 저장소 연동: 세션, 게임 기록, 사용자 설정.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use bevy::app::AppExit;
use bevy::prelude::*;
use fruit_star_config::{ConfigSet, Language, Quality};
use fruit_star_core::{RandomSource, ReelTiming};
use fruit_star_game::{
    GameController, GameError, GameEvent, MachineProfile, PlayMode, SystemRandom,
};
use fruit_star_machine::{
    AppState, Machine, MachineEvent, MachineSet, Notice, Paused, SessionCommand, Settings,
    StatsSnapshot, ViewMode,
};
use fruit_star_storage::{Store, StoreError, default_database_path};

/// 저장소 리소스. SQLite 연결은 스레드 사이에서 공유할 수 없으므로 뮤텍스로 감싼다.
#[derive(Resource)]
pub struct Persistence {
    store: Mutex<Store>,
    /// 디스크에 저장하는지 여부 (열지 못해 메모리 저장소로 대체했다면 거짓).
    pub persistent: bool,
    /// 현재 플레이 세션 ID.
    session: Option<i64>,
}

impl Persistence {
    fn store(&self) -> MutexGuard<'_, Store> {
        // 다른 스레드가 패닉으로 락을 오염시켰더라도 데이터베이스 자체는 트랜잭션으로
        // 보호되므로 계속 사용한다.
        self.store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 저장소를 연다. 실패하면 메모리 저장소로 대체하고 경고를 돌려준다.
pub fn open_store(database: Option<&str>) -> Result<(Persistence, Vec<String>), StoreError> {
    let mut warnings = Vec::new();
    let opened = match database {
        Some(":memory:") => None,
        Some(path) => Some(Store::open(Path::new(path))),
        None => default_database_path().map(|path| Store::open(&path)),
    };
    let (mut store, persistent) = match opened {
        Some(Ok(store)) => (store, true),
        Some(Err(error)) => {
            warnings.push(format!(
                "기록을 저장할 수 없습니다 ({error}). 이번 실행의 기록은 종료하면 사라집니다."
            ));
            (Store::in_memory()?, false)
        }
        None => (Store::in_memory()?, false),
    };
    let report = store.recover()?;
    if report.unclean_sessions > 0 {
        warnings.push(format!(
            "지난 실행이 정상 종료되지 않아 세션 {}개를 정리했습니다.",
            report.unclean_sessions
        ));
    }
    if report.balance_repaired {
        warnings.push("보유 메달 값을 원장 기록으로부터 복구했습니다.".to_owned());
    }
    Ok((
        Persistence {
            store: Mutex::new(store),
            persistent,
            session: None,
        },
        warnings,
    ))
}

const KEY_LANGUAGE: &str = "game.language";
const KEY_MASTER: &str = "audio.master";
const KEY_EFFECTS: &str = "audio.effects";
const KEY_AMBIENT: &str = "audio.ambient";
const KEY_MUTED: &str = "audio.muted";
const KEY_QUALITY: &str = "graphics.quality";
const KEY_LED: &str = "graphics.led_brightness";
const KEY_REDUCE: &str = "graphics.reduce_flashing";

/// 저장된 사용자 설정을 설정 파일 값 위에 덮어쓴다. 범위를 벗어난 값은 무시한다.
pub fn apply_saved_settings(persistence: &Persistence, config: &mut ConfigSet) {
    let store = persistence.store();
    let read = |key: &str| store.setting(key).ok().flatten();
    let unit = |key: &str| {
        read(key)
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| (0.0..=1.0).contains(v))
    };
    if let Some(language) = read(KEY_LANGUAGE).and_then(|v| Language::from_key(&v)) {
        config.application.game.language = language;
    }
    if let Some(value) = unit(KEY_MASTER) {
        config.audio.master = value;
    }
    if let Some(value) = unit(KEY_EFFECTS) {
        config.audio.effects = value;
    }
    if let Some(value) = unit(KEY_AMBIENT) {
        config.audio.ambient = value;
    }
    if let Some(value) = read(KEY_MUTED).and_then(|v| v.parse().ok()) {
        config.audio.muted = value;
    }
    if let Some(quality) = read(KEY_QUALITY).and_then(|v| Quality::from_key(&v)) {
        config.graphics.render.quality = quality;
    }
    if let Some(value) = read(KEY_LED)
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| (0.0..=1.5).contains(v))
    {
        config.graphics.led.brightness = value;
    }
    if let Some(value) = read(KEY_REDUCE).and_then(|v| v.parse().ok()) {
        config.graphics.led.reduce_flashing = value;
    }
}

/// 난수 시드 (재현 가능한 실행용). 없으면 운영체제 엔트로피를 쓴다.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct RandomSeed(pub Option<u64>);

fn random_source(seed: RandomSeed) -> Box<dyn RandomSource + Send + Sync> {
    match seed.0 {
        Some(seed) => Box::new(SystemRandom::from_seed(seed)),
        None => Box::new(SystemRandom::from_os()),
    }
}

/// 지정한 모드의 게임 컨트롤러를 만든다. Classic 모드는 저장된 보유 메달로 시작한다.
pub fn build_controller(
    persistence: &Persistence,
    config: &ConfigSet,
    profile: MachineProfile,
    timing: ReelTiming,
    mode: PlayMode,
    seed: RandomSeed,
) -> Result<GameController, GameError> {
    let store = persistence.store();
    let game = &config.application.game;
    let credit = match mode {
        PlayMode::Classic => store
            .classic_balance()
            .ok()
            .flatten()
            .unwrap_or(game.initial_credit),
        PlayMode::FreePlay | PlayMode::Practice => 0,
    };
    let first_game = store.next_game_id().unwrap_or(1);
    GameController::new(
        profile,
        timing,
        mode,
        credit,
        game.credit_limit.max(credit),
        first_game,
        random_source(seed),
    )
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, refresh_stats)
            .add_systems(
                Update,
                (handle_session_commands, record_events, save_settings)
                    .chain()
                    .in_set(MachineSet::React),
            )
            .add_systems(OnExit(AppState::InGame), end_session)
            .add_systems(Last, close_on_exit);
    }
}

fn snapshot(persistence: &Persistence) -> StatsSnapshot {
    let store = persistence.store();
    let stats = store.statistics().unwrap_or_default();
    StatsSnapshot {
        games: stats.games,
        play_seconds: stats.play_seconds,
        total_bet: stats.total_bet,
        total_payout: stats.total_payout,
        replays: stats.replays,
        big_bonuses: stats.big_bonuses,
        regular_bonuses: stats.regular_bonuses,
        max_payout: stats.max_payout,
        achievements: store
            .achievements()
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.key().to_owned())
            .collect(),
        persistent: persistence.persistent,
    }
}

fn refresh_stats(persistence: Res<Persistence>, mut stats: ResMut<StatsSnapshot>) {
    *stats = snapshot(&persistence);
}

fn handle_session_commands(
    mut commands: MessageReader<SessionCommand>,
    mut persistence: ResMut<Persistence>,
    mut machine: ResMut<Machine>,
    settings: Res<Settings>,
    seed: Res<RandomSeed>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut view: ResMut<ViewMode>,
    mut paused: ResMut<Paused>,
    mut stats: ResMut<StatsSnapshot>,
    mut notices: MessageWriter<Notice>,
    mut exit: MessageWriter<AppExit>,
) {
    for command in commands.read() {
        match *command {
            SessionCommand::Start {
                mode,
                view: wanted_view,
            } => {
                if *state.get() != AppState::MainMenu {
                    continue;
                }
                let profile = machine.profile().clone();
                let timing = settings.config.machine.reel_timing();
                match build_controller(&persistence, &settings.config, profile, timing, mode, *seed)
                {
                    Ok(controller) => {
                        let credit = controller.credit();
                        machine.0 = controller;
                        let session = {
                            let mut store = persistence.store();
                            // 세션은 기종 정보를 참조하므로 먼저 등록(갱신)한다.
                            store.upsert_machine(machine.profile()).and_then(|()| {
                                store.begin_session(&machine.profile().id, mode, credit)
                            })
                        };
                        match session {
                            Ok(id) => persistence.session = Some(id),
                            Err(error) => {
                                error!("세션을 시작할 수 없습니다: {error}");
                                persistence.session = None;
                                notices.write(Notice {
                                    key: "notice.storage_error".to_owned(),
                                });
                            }
                        }
                        *view = wanted_view;
                        paused.0 = false;
                        next.set(AppState::InGame);
                    }
                    Err(error) => error!("게임기를 준비할 수 없습니다: {error}"),
                }
            }
            SessionCommand::End => {
                if *state.get() == AppState::InGame {
                    paused.0 = false;
                    next.set(AppState::MainMenu);
                }
            }
            SessionCommand::ResetHistory => {
                if *state.get() != AppState::MainMenu {
                    continue;
                }
                let result = persistence.store().reset_history();
                match result {
                    Ok(()) => {
                        *stats = snapshot(&persistence);
                        notices.write(Notice {
                            key: "notice.history_reset".to_owned(),
                        });
                    }
                    Err(error) => {
                        error!("기록을 초기화할 수 없습니다: {error}");
                        notices.write(Notice {
                            key: "notice.storage_error".to_owned(),
                        });
                    }
                }
            }
            SessionCommand::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

/// 게임 결과와 원장 변경을 기록한다.
fn record_events(
    mut events: MessageReader<MachineEvent>,
    persistence: Res<Persistence>,
    machine: Res<Machine>,
    mut stats: ResMut<StatsSnapshot>,
    mut notices: MessageWriter<Notice>,
) {
    let Some(session) = persistence.session else {
        events.clear();
        return;
    };
    let mut changed = false;
    for MachineEvent(event) in events.read() {
        let result = match event {
            GameEvent::MedalInserted { entry } => persistence
                .store()
                .record_credit(session, machine.mode(), entry)
                .map(|()| Vec::new()),
            GameEvent::GameCompleted(outcome) => persistence.store().record_game(session, outcome),
            _ => continue,
        };
        changed = true;
        match result {
            Ok(unlocked) => {
                for achievement in unlocked {
                    notices.write(Notice {
                        key: format!("achievement.{}", achievement.key()),
                    });
                }
            }
            Err(error) => {
                error!("기록 저장 실패: {error}");
                notices.write(Notice {
                    key: "notice.storage_error".to_owned(),
                });
            }
        }
    }
    if changed {
        *stats = snapshot(&persistence);
    }
}

/// 세션을 닫을 때 기록할 잔액.
///
/// 게임 도중에 나가면 그 게임의 베팅은 기록되지 않고 버려지므로(다시 들어오면 베팅 전 잔액으로
/// 시작한다), Classic 모드에서는 메모리의 잔액이 아니라 저장된 보유 메달을 기록해 둘을 일치시킨다.
fn closing_balance(persistence: &Persistence, machine: &Machine) -> u32 {
    if machine.mode() == PlayMode::Classic
        && let Ok(Some(saved)) = persistence.store().classic_balance()
    {
        return saved;
    }
    machine.credit()
}

/// 열려 있는 세션을 닫는다. 닫은 세션이 있었으면 참을 돌려준다.
fn close_session(persistence: &mut Persistence, machine: &Machine) -> bool {
    let Some(session) = persistence.session.take() else {
        return false;
    };
    // 잔액을 먼저 구한 뒤에 저장소를 잠근다. 한 문장 안에서 두 번 잠그면 교착 상태가 된다.
    let balance = closing_balance(persistence, machine);
    let result = persistence.store().end_session(session, balance);
    if let Err(error) = result {
        error!("세션을 닫을 수 없습니다: {error}");
    }
    true
}

fn end_session(
    mut persistence: ResMut<Persistence>,
    machine: Res<Machine>,
    mut stats: ResMut<StatsSnapshot>,
) {
    if close_session(&mut persistence, &machine) {
        *stats = snapshot(&persistence);
    }
}

/// 프로그램이 끝날 때 열린 세션을 정상 종료로 닫는다.
pub(crate) fn close_on_exit(
    mut exits: MessageReader<AppExit>,
    mut persistence: ResMut<Persistence>,
    machine: Res<Machine>,
) {
    if exits.read().next().is_none() {
        return;
    }
    if close_session(&mut persistence, &machine) {}
}

/// 사용자가 바꾼 설정을 저장해 다음 실행에서 복원되게 한다.
fn save_settings(settings: Res<Settings>, persistence: Res<Persistence>) {
    if !settings.is_changed() || settings.is_added() {
        return;
    }
    let config = &settings.config;
    let values = [
        (
            KEY_LANGUAGE,
            config.application.game.language.key().to_owned(),
        ),
        (KEY_MASTER, config.audio.master.to_string()),
        (KEY_EFFECTS, config.audio.effects.to_string()),
        (KEY_AMBIENT, config.audio.ambient.to_string()),
        (KEY_MUTED, config.audio.muted.to_string()),
        (KEY_QUALITY, config.graphics.render.quality.key().to_owned()),
        (KEY_LED, config.graphics.led.brightness.to_string()),
        (KEY_REDUCE, config.graphics.led.reduce_flashing.to_string()),
    ];
    let mut store = persistence.store();
    for (key, value) in values {
        // 값이 그대로면 쓰지 않는다.
        if store.setting(key).ok().flatten().as_deref() == Some(value.as_str()) {
            continue;
        }
        if let Err(error) = store.set_setting(key, &value) {
            error!("설정을 저장할 수 없습니다: {error}");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Classic 세션을 게임 도중(베팅만 한 상태)에 닫아도 멈추지 않고, 두 번 닫아도 안전하다.
    /// 저장소 잠금을 한 문장에서 두 번 잡아 교착 상태가 되던 회귀를 막는다.
    #[test]
    fn closing_a_classic_session_mid_game_does_not_deadlock() {
        let Ok((mut persistence, _)) = open_store(Some(":memory:")) else {
            panic!("메모리 저장소를 열 수 없다");
        };
        let config = ConfigSet::default();
        let Ok(profile) = config.machine.to_profile() else {
            panic!("기본 기종 설정이 올바르지 않다");
        };
        let Ok(mut controller) = build_controller(
            &persistence,
            &config,
            profile,
            config.machine.reel_timing(),
            PlayMode::Classic,
            RandomSeed(Some(1)),
        ) else {
            panic!("컨트롤러를 만들 수 없다");
        };
        let session = {
            let mut store = persistence.store();
            assert!(store.upsert_machine(controller.profile()).is_ok());
            store.begin_session("fruit_classic", PlayMode::Classic, controller.credit())
        };
        persistence.session = session.ok();
        assert!(persistence.session.is_some());
        assert!(controller.press_bet().is_ok());

        let machine = Machine(controller);
        assert!(close_session(&mut persistence, &machine));
        assert!(persistence.session.is_none());
        assert!(!close_session(&mut persistence, &machine));
    }
}
