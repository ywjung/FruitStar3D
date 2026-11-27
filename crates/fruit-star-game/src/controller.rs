//! 게임 진행 컨트롤러.
//!
//! 게임기 한 대의 논리 상태 전체를 가진다. 입력 메서드(`press_bet`, `pull_lever`,
//! `press_stop` 등)와 시간 진행(`tick`)만으로 구동되며, 일어난 일은
//! [`GameEvent`]로 쌓인다. 메달 원장은 이 컨트롤러 안에서만 변경된다.

use fruit_star_core::{
    CreditLedger, GameState, LedgerEntry, REEL_COUNT, RandomSource, Reel, ReelState, ReelTiming,
};

use crate::bonus::{BonusKind, BonusState};
use crate::control::{Allowed, choose_stop_lines};
use crate::error::GameError;
use crate::evaluator::{active_lines, evaluate_lines, windows_at};
use crate::events::{GameEvent, GameOutcome};
use crate::lottery::Flag;
use crate::profile::MachineProfile;

/// 플레이 모드.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlayMode {
    /// 보유 메달로 플레이하는 기본 모드.
    #[default]
    Classic,
    /// 메달이 자동으로 채워지는 모드.
    FreePlay,
    /// 메달이 자동으로 채워지고 내부 추첨 결과를 볼 수 있는 연습 모드.
    Practice,
}

impl PlayMode {
    pub const ALL: [PlayMode; 3] = [PlayMode::Classic, PlayMode::FreePlay, PlayMode::Practice];

    pub const fn key(self) -> &'static str {
        match self {
            PlayMode::Classic => "classic",
            PlayMode::FreePlay => "free_play",
            PlayMode::Practice => "practice",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == key)
    }

    /// 메달이 부족할 때 자동으로 채워 주는 모드인지 여부.
    pub const fn auto_refill(self) -> bool {
        matches!(self, PlayMode::FreePlay | PlayMode::Practice)
    }
}

/// 한 게임 안에서의 진행 단계.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Phase {
    /// 베팅과 레버 입력을 기다린다.
    #[default]
    Ready,
    /// 세 릴이 모두 돌고 있다.
    Spinning,
    /// 하나 이상의 릴에 정지 입력이 들어갔다.
    Stopping,
    /// 조합 평가와 정산 중.
    Evaluating,
    /// 결과 표시와 메달 지급 중.
    Result,
}

/// 진행 중인 게임 한 판의 상태.
#[derive(Debug, Clone)]
struct ActiveGame {
    game_id: u64,
    flag: Flag,
    allowed: Allowed,
    replay_game: bool,
    in_bonus: bool,
    bet: u32,
    lines: usize,
    bet_entries: Vec<LedgerEntry>,
    targets: [Option<usize>; REEL_COUNT],
    slips: [usize; REEL_COUNT],
    stop_order: Vec<usize>,
}

/// 자동 충전 모드에서 한 번에 채우는 메달 수.
const AUTO_REFILL_AMOUNT: u32 = 50;

/// 게임 진행 컨트롤러.
pub struct GameController {
    profile: MachineProfile,
    reels: [Reel; REEL_COUNT],
    ledger: CreditLedger,
    rng: Box<dyn RandomSource + Send + Sync>,
    mode: PlayMode,
    phase: Phase,
    /// 다음에 시작할 게임의 ID.
    next_game_id: u64,
    /// 베팅이 놓였는지 여부와 그 원장 항목.
    /// 다음 게임에 걸린 메달 수 (0이면 아직 베팅하지 않음).
    bet_amount: u32,
    pending_bet_entries: Vec<LedgerEntry>,
    /// 다음 게임이 리플레이(무료)인지 여부.
    replay_pending: bool,
    /// 리플레이로 다시 걸릴 메달 수 (리플레이가 성립한 게임의 베팅 수).
    replay_bet: u32,
    carried_bonus: Option<BonusKind>,
    /// STAR CHANCE 램프 점등 여부. 보너스 당첨 게임에서 레버를 당길 때(일부) 또는 그 게임이 끝날 때 켜진다.
    chance_lamp: bool,
    bonus: Option<BonusState>,
    active: Option<ActiveGame>,
    last_outcome: Option<GameOutcome>,
    result_timer: f64,
    forced_flag: Option<Flag>,
    events: Vec<GameEvent>,
}

impl std::fmt::Debug for GameController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameController")
            .field("profile", &self.profile.id)
            .field("mode", &self.mode)
            .field("phase", &self.phase)
            .field("balance", &self.ledger.balance())
            .field("next_game_id", &self.next_game_id)
            .finish_non_exhaustive()
    }
}

impl GameController {
    /// 컨트롤러를 만든다. `first_game_id`는 저장된 기록에 이어서 번호를 매기기 위한 값이다.
    pub fn new(
        profile: MachineProfile,
        timing: ReelTiming,
        mode: PlayMode,
        initial_credit: u32,
        credit_limit: u32,
        first_game_id: u64,
        rng: Box<dyn RandomSource + Send + Sync>,
    ) -> Result<Self, GameError> {
        profile.validate()?;
        let reels = [
            Reel::new(0, profile.strips[0].clone(), timing)?,
            Reel::new(1, profile.strips[1].clone(), timing)?,
            Reel::new(2, profile.strips[2].clone(), timing)?,
        ];
        Ok(Self {
            profile,
            reels,
            ledger: CreditLedger::new(initial_credit, credit_limit),
            rng,
            mode,
            phase: Phase::Ready,
            next_game_id: first_game_id.max(1),
            bet_amount: 0,
            pending_bet_entries: Vec::new(),
            replay_pending: false,
            replay_bet: 0,
            carried_bonus: None,
            chance_lamp: false,
            bonus: None,
            active: None,
            last_outcome: None,
            result_timer: 0.0,
            forced_flag: None,
            events: Vec::new(),
        })
    }

    // ---- 조회 ----------------------------------------------------------

    pub fn profile(&self) -> &MachineProfile {
        &self.profile
    }

    pub fn reels(&self) -> &[Reel; REEL_COUNT] {
        &self.reels
    }

    pub fn ledger(&self) -> &CreditLedger {
        &self.ledger
    }

    pub fn credit(&self) -> u32 {
        self.ledger.balance()
    }

    pub fn mode(&self) -> PlayMode {
        self.mode
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// 레버를 당길 수 있을 만큼 베팅되어 있는지 여부 (1메달 이상).
    pub fn bet_placed(&self) -> bool {
        self.bet_amount > 0
    }

    /// 다음 게임에 걸린 메달 수.
    pub fn bet_amount(&self) -> u32 {
        self.bet_amount
    }

    /// 계기판에 표시할 베팅. 결과를 보는 동안에는 방금 정산한 판의 베팅을 유지한다.
    pub fn displayed_bet(&self) -> u32 {
        if let Some(active) = &self.active {
            active.bet
        } else if self.phase == Phase::Result {
            self.last_outcome.as_ref().map_or(0, |outcome| outcome.bet)
        } else {
            self.bet_amount
        }
    }

    /// 한 게임에 걸 수 있는 최대 메달 수 (MAX BET).
    pub fn max_bet(&self) -> u32 {
        self.profile.bet
    }

    /// 메달을 더 걸 수 있는지 여부.
    pub fn can_add_bet(&self) -> bool {
        self.phase == Phase::Ready && self.bet_amount < self.profile.bet
    }

    /// 지금 표시해야 할 유효 라인 수: 진행 중인 게임이 있으면 그 게임의 라인 수,
    /// 없으면 현재 걸린 메달에 해당하는 라인 수.
    pub fn lit_lines(&self) -> usize {
        match &self.active {
            Some(active) => active.lines,
            None => active_lines(self.bet_amount),
        }
    }

    /// 다음 게임이 리플레이(무료)인지 여부.
    pub fn replay_pending(&self) -> bool {
        self.replay_pending
    }

    pub fn bonus(&self) -> Option<&BonusState> {
        self.bonus.as_ref()
    }

    /// STAR CHANCE 램프가 켜져 있는지 여부.
    ///
    /// 보너스에 당첨된 게임에서 조합을 맞추지 못하고 끝났을 때 켜지고, 보너스가
    /// 시작되면 꺼진다. 릴이 도는 동안에는 당첨 사실을 드러내지 않는다.
    pub fn chance_lamp(&self) -> bool {
        self.chance_lamp
    }

    pub fn last_outcome(&self) -> Option<&GameOutcome> {
        self.last_outcome.as_ref()
    }

    /// 지금까지 완료한 게임에 이어 다음에 부여될 게임 ID.
    pub fn next_game_id(&self) -> u64 {
        self.next_game_id
    }

    /// 연습 모드에서만 공개되는 이번 게임의 허용 역할 정보.
    pub fn practice_hint(&self) -> Option<(Flag, Option<BonusKind>)> {
        if self.mode != PlayMode::Practice {
            return None;
        }
        // 게임 중에는 이번 게임의 추첨 결과, 게임 사이에는 직전 게임의 결과를 보여 준다.
        let flag = match (&self.active, &self.last_outcome) {
            (Some(active), _) => active.flag,
            (None, Some(outcome)) => outcome.flag,
            (None, None) => Flag::Blank,
        };
        Some((flag, self.carried_bonus))
    }

    /// 릴 `reel`이 정지 입력을 받을 수 있는지 여부.
    pub fn can_stop(&self, reel: usize) -> bool {
        matches!(self.phase, Phase::Spinning | Phase::Stopping)
            && self
                .reels
                .get(reel)
                .is_some_and(|r| r.state() == ReelState::Spinning)
    }

    /// 화면 전환을 제외한, 플레이 진행 관점의 게임 상태.
    pub fn game_state(&self) -> GameState {
        match self.phase {
            Phase::Ready if self.bonus.is_some() => GameState::Bonus,
            Phase::Ready => GameState::Ready,
            Phase::Spinning => GameState::Spinning,
            Phase::Stopping => GameState::Stopping,
            Phase::Evaluating => GameState::Evaluating,
            Phase::Result => GameState::Result,
        }
    }

    /// 쌓인 이벤트를 꺼낸다.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    // ---- 입력 ----------------------------------------------------------

    /// 전원 투입 이벤트를 발생시킨다.
    pub fn power_on(&mut self) {
        self.events.push(GameEvent::PowerOn);
        if self.chance_lamp() {
            self.events.push(GameEvent::ChanceLampChanged(true));
        }
    }

    /// 전원 차단 이벤트를 발생시킨다.
    pub fn power_off(&mut self) {
        self.events.push(GameEvent::PowerOff);
    }

    /// 플레이 모드를 바꾼다. 게임이 진행 중이면 거부한다.
    pub fn set_mode(&mut self, mode: PlayMode) -> Result<(), GameError> {
        if self.phase != Phase::Ready {
            return Err(GameError::WrongPhase { phase: self.phase });
        }
        self.mode = mode;
        if mode != PlayMode::Practice {
            self.forced_flag = None;
        }
        Ok(())
    }

    /// 다음 게임의 추첨 결과를 지정한다. 연습 모드에서만 적용된다.
    pub fn set_forced_flag(&mut self, flag: Option<Flag>) {
        self.forced_flag = if self.mode == PlayMode::Practice {
            flag
        } else {
            None
        };
    }

    /// 가상 메달을 투입한다.
    pub fn insert_medals(&mut self, amount: u32) -> Result<(), GameError> {
        let entry = self.ledger.insert(amount)?;
        self.events.push(GameEvent::MedalInserted { entry });
        Ok(())
    }

    /// MAX BET 버튼: 최대 베팅까지 한 번에 건다. 이미 일부가 걸려 있으면 나머지만 더한다.
    pub fn press_bet(&mut self) -> Result<(), GameError> {
        let missing = self.profile.bet.saturating_sub(self.bet_amount);
        self.add_bet(missing)
    }

    /// 1 BET 버튼: 메달을 한 장 건다. 최대 베팅에 이르면 거부한다.
    pub fn press_bet_one(&mut self) -> Result<(), GameError> {
        self.add_bet(1)
    }

    fn add_bet(&mut self, amount: u32) -> Result<(), GameError> {
        if self.phase != Phase::Ready {
            return Err(GameError::WrongPhase { phase: self.phase });
        }
        if amount == 0 || self.bet_amount + amount > self.profile.bet {
            return Err(GameError::AlreadyBet);
        }
        if self.mode.auto_refill() && self.ledger.balance() < amount {
            self.insert_medals(AUTO_REFILL_AMOUNT.max(amount))?;
        }
        let entry = self.ledger.bet(self.next_game_id, amount)?;
        self.pending_bet_entries.push(entry);
        self.bet_amount += amount;
        self.events.push(GameEvent::BetPressed {
            amount,
            total: self.bet_amount,
            replay: false,
        });
        Ok(())
    }

    /// START 레버.
    pub fn pull_lever(&mut self) -> Result<(), GameError> {
        if self.phase != Phase::Ready {
            return Err(GameError::WrongPhase { phase: self.phase });
        }
        if self.bet_amount == 0 {
            return Err(GameError::BetRequired);
        }
        if let Some(reel) = self.reels.iter().find(|r| r.is_moving()) {
            return Err(GameError::Reel(fruit_star_core::ReelError::CannotStart {
                reel: reel.id(),
                state: reel.state(),
            }));
        }

        let in_bonus = self.bonus.is_some();
        let (bet, max_bet) = (self.bet_amount, self.profile.bet);
        let table = if in_bonus {
            &self.profile.bonus.lottery
        } else {
            &self.profile.lottery
        };
        // 당첨 확률은 베팅 수에 비례한다 (배당은 같으므로 메달당 기대 환수가 같다).
        let mut flag = table.draw_for_bet(self.rng.as_mut(), bet, max_bet);
        if let Some(forced) = self.forced_flag.take()
            && self.mode == PlayMode::Practice
            && !in_bonus
        {
            flag = forced;
        }
        // 보너스가 이미 이월 중이거나 진행 중이면 보너스 당첨은 꽝으로 취급한다.
        if matches!(flag, Flag::Bonus(_)) && (self.carried_bonus.is_some() || in_bonus) {
            flag = Flag::Blank;
        }
        // 새로 당첨된 보너스는 일부를 레버를 당기는 순간 램프로 알린다 (선고지). 나머지는 릴이 모두 멈춘 뒤.
        let lamp_on_lever = if let Flag::Bonus(kind) = flag {
            self.carried_bonus = Some(kind);
            self.rng.below(100) < self.profile.bonus.lamp_lead_percent
        } else {
            false
        };
        let allowed = Allowed::from_flag(flag, if in_bonus { None } else { self.carried_bonus });

        let game_id = self.next_game_id;
        for reel in &mut self.reels {
            reel.start()?;
        }
        self.active = Some(ActiveGame {
            game_id,
            flag,
            allowed,
            replay_game: self.replay_pending,
            in_bonus,
            bet,
            lines: active_lines(bet),
            bet_entries: std::mem::take(&mut self.pending_bet_entries),
            targets: [None; REEL_COUNT],
            slips: [0; REEL_COUNT],
            stop_order: Vec::with_capacity(REEL_COUNT),
        });
        self.bet_amount = 0;
        self.replay_pending = false;
        self.phase = Phase::Spinning;
        self.events.push(GameEvent::LeverPulled { game_id });
        if lamp_on_lever && !self.chance_lamp {
            self.chance_lamp = true;
            self.events.push(GameEvent::ChanceLampChanged(true));
        }
        for reel in 0..REEL_COUNT {
            self.events.push(GameEvent::ReelStarted(reel));
        }
        Ok(())
    }

    /// 정지 버튼. 이미 정지 입력을 받은 릴이나 아직 정속에 이르지 않은 릴에 대한
    /// 입력은 거부되며 상태를 바꾸지 않는다.
    pub fn press_stop(&mut self, reel: usize) -> Result<(), GameError> {
        if reel >= REEL_COUNT {
            return Err(GameError::InvalidReel { reel });
        }
        if !matches!(self.phase, Phase::Spinning | Phase::Stopping) {
            return Err(GameError::WrongPhase { phase: self.phase });
        }
        let Some(active) = self.active.as_mut() else {
            return Err(GameError::WrongPhase { phase: self.phase });
        };
        if active.targets[reel].is_some() || self.reels[reel].state() != ReelState::Spinning {
            return Err(GameError::StopNotReady { reel });
        }
        let pressed = self.reels[reel].next_index();
        let decision = choose_stop_lines(
            &self.profile,
            active.allowed,
            active.lines,
            active.targets,
            reel,
            pressed,
        );
        self.reels[reel].request_stop(decision.index)?;
        active.targets[reel] = Some(decision.index);
        active.slips[reel] = decision.slip;
        active.stop_order.push(reel);
        self.phase = Phase::Stopping;
        self.events.push(GameEvent::StopPressed(reel));
        Ok(())
    }

    // ---- 시간 진행 ------------------------------------------------------

    /// `dt`초만큼 시간을 진행한다.
    pub fn tick(&mut self, dt: f64) {
        for (index, reel) in self.reels.iter_mut().enumerate() {
            let update = reel.update(dt);
            if update.locked {
                self.events.push(GameEvent::ReelStopped {
                    reel: index,
                    index: reel.current_index(),
                });
            }
        }

        match self.phase {
            Phase::Stopping => {
                if self.reels.iter().all(|r| r.state() == ReelState::Stopped) {
                    self.phase = Phase::Evaluating;
                    self.settle_game();
                }
            }
            Phase::Result => {
                self.result_timer -= dt.max(0.0);
                if self.result_timer <= 0.0 {
                    self.finish_result();
                }
            }
            Phase::Ready | Phase::Spinning | Phase::Evaluating => {}
        }
    }

    /// 조합을 평가하고 정산한다. 한 게임에 정확히 한 번만 실행된다.
    fn settle_game(&mut self) {
        let Some(active) = self.active.take() else {
            // 진행 중인 게임 없이 평가 단계에 올 수는 없다. 안전하게 대기로 돌아간다.
            self.phase = Phase::Ready;
            return;
        };
        let stops = [
            self.reels[0].current_index(),
            self.reels[1].current_index(),
            self.reels[2].current_index(),
        ];
        let windows = windows_at(&self.profile, stops);
        let evaluation = evaluate_lines(&self.profile, &windows, active.lines);

        // 원장이 이미 정산을 기록했다면 중복 정산이므로 지급하지 않는다.
        let payout_entry = self
            .ledger
            .settle(active.game_id, evaluation.payout)
            .unwrap_or_default();
        let payout = payout_entry.map_or(0, |e| e.amount);

        let mut stop_order = [0usize; REEL_COUNT];
        for (slot, reel) in stop_order.iter_mut().zip(active.stop_order.iter()) {
            *slot = *reel;
        }

        let mut bonus_completed = None;
        if let Some(bonus) = self.bonus.as_mut()
            && bonus.record_game(payout)
        {
            bonus_completed = Some(*bonus);
            self.bonus = None;
        }

        let mut bonus_started = None;
        if !active.in_bonus
            && let Some(kind) = evaluation.bonus
        {
            self.carried_bonus = None;
            self.bonus = Some(BonusState::new(kind, self.profile.bonus.payout_cap(kind)));
            bonus_started = Some(kind);
        }

        self.replay_pending = evaluation.replay;
        self.replay_bet = active.bet;

        let outcome = GameOutcome {
            game_id: active.game_id,
            mode: self.mode,
            bet: active.bet,
            lines: active.lines,
            replay_game: active.replay_game,
            in_bonus: active.in_bonus,
            flag: active.flag,
            stops,
            slips: active.slips,
            stop_order,
            windows,
            wins: evaluation.wins,
            payout,
            replay_awarded: evaluation.replay,
            bonus_started,
            balance_after: self.ledger.balance(),
            bet_entries: active.bet_entries,
            payout_entry,
        };

        self.next_game_id = active.game_id + 1;
        self.result_timer = self.profile.flow.result_hold_s
            + f64::from(payout) * self.profile.flow.payout_interval_s;
        self.phase = Phase::Result;
        self.last_outcome = Some(outcome.clone());

        self.events
            .push(GameEvent::GameCompleted(Box::new(outcome)));
        if payout > 0 {
            self.events.push(GameEvent::MedalDispensed(payout));
        }
        if let Some(kind) = bonus_started {
            if self.chance_lamp {
                self.chance_lamp = false;
                self.events.push(GameEvent::ChanceLampChanged(false));
            }
            self.events.push(GameEvent::BonusStarted(kind));
        } else if matches!(active.flag, Flag::Bonus(_)) && !self.chance_lamp {
            // 보너스에 당첨된 게임에서 맞추지 못했다면 램프로 알린다 (후고지).
            self.chance_lamp = true;
            self.events.push(GameEvent::ChanceLampChanged(true));
        }
        if let Some(finished) = bonus_completed {
            self.events.push(GameEvent::BonusCompleted {
                kind: finished.kind,
                paid: finished.paid,
                games: finished.games,
            });
        }
    }

    /// 결과 표시를 끝내고 다음 게임을 받을 준비를 한다.
    fn finish_result(&mut self) {
        self.phase = Phase::Ready;
        self.result_timer = 0.0;
        if self.replay_pending {
            // 리플레이: 메달 차감 없이 같은 베팅이 자동으로 놓인다.
            self.bet_amount = self.replay_bet.clamp(1, self.profile.bet);
            self.pending_bet_entries.clear();
            self.events.push(GameEvent::BetPressed {
                amount: self.bet_amount,
                total: self.bet_amount,
                replay: true,
            });
        }
    }
}
