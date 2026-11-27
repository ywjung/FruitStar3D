#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use fruit_star_core::{RandomSource, ReelTiming, SplitMix64};
use fruit_star_game::{GameController, GameEvent, GameOutcome, MachineProfile, Phase, PlayMode};

/// 미리 정한 값을 순서대로 돌려주는 난수 공급원. 값이 떨어지면 꽝이 나오는 큰 값을 준다.
pub struct Scripted {
    values: Vec<u32>,
    cursor: usize,
}

impl Scripted {
    /// 추첨 분모(65536) 기준의 값 목록으로 만든다.
    pub fn rolls(rolls: &[u32]) -> Box<Self> {
        Box::new(Self {
            // `below(65536)`은 상위 16비트를 쓰므로 그 자리에 값을 넣는다.
            // 하위 비트를 채워 두면 범위가 65536의 배수가 아닌 추첨(1·2메달 게임)에서도
            // 기각 샘플링에 걸리지 않고 그대로 쓰인다.
            values: rolls.iter().map(|r| (r << 16) | 0xFFFF).collect(),
            cursor: 0,
        })
    }
}

impl RandomSource for Scripted {
    fn next_u32(&mut self) -> u32 {
        let value = self.values.get(self.cursor).copied().unwrap_or(u32::MAX);
        self.cursor += 1;
        value
    }
}

pub fn controller_with(
    mode: PlayMode,
    credit: u32,
    rng: Box<dyn RandomSource + Send + Sync>,
) -> GameController {
    GameController::new(
        MachineProfile::fruit_classic(),
        ReelTiming::default(),
        mode,
        credit,
        9_999,
        1,
        rng,
    )
    .unwrap()
}

pub fn seeded(mode: PlayMode, credit: u32, seed: u64) -> GameController {
    controller_with(mode, credit, Box::new(SplitMix64::new(seed)))
}

/// 추첨표에서 `flag`가 뽑히는 구간의 시작 값을 구한다.
pub fn roll_for(profile: &MachineProfile, flag: fruit_star_game::Flag) -> u32 {
    let mut start = 0;
    for (f, weight) in profile.lottery.entries() {
        if *f == flag {
            return start;
        }
        start += weight;
    }
    panic!("추첨표에 {flag:?} 항목이 없습니다");
}

/// 릴이 정속에 이를 때까지 진행한다.
pub fn spin_up(game: &mut GameController) {
    game.tick(0.4);
    assert_eq!(game.phase(), Phase::Spinning);
}

/// 릴 `reel`의 다음 도달 인덱스가 `index`가 되도록 시간을 진행한다.
pub fn aim(game: &mut GameController, reel: usize, index: usize) {
    let r = &game.reels()[reel];
    let len = r.len() as f64;
    let speed = r.timing().spin_speed;
    // 목표 인덱스의 반 칸 앞까지 진행하면 next_index가 목표가 된다.
    let distance = (index as f64 - 0.5 - r.position()).rem_euclid(len);
    game.tick(distance / speed);
    assert_eq!(game.reels()[reel].next_index(), index);
}

/// 모든 릴이 멈추고 결과 표시가 끝날 때까지 진행하고 그 사이의 이벤트를 돌려준다.
pub fn finish(game: &mut GameController) -> Vec<GameEvent> {
    let mut events = Vec::new();
    for _ in 0..2_000 {
        game.tick(0.02);
        events.extend(game.drain_events());
        if game.phase() == Phase::Ready {
            return events;
        }
    }
    panic!("게임이 끝나지 않았습니다: {:?}", game.phase());
}

pub fn outcome_of(events: &[GameEvent]) -> GameOutcome {
    events
        .iter()
        .find_map(|e| match e {
            GameEvent::GameCompleted(outcome) => Some((**outcome).clone()),
            _ => None,
        })
        .expect("GameCompleted 이벤트가 있어야 한다")
}

/// 지정한 위치를 노려 한 게임을 끝까지 진행한다. `presses`는 (릴, 노릴 인덱스) 순서쌍이다.
pub fn play_aimed(
    game: &mut GameController,
    presses: [(usize, usize); 3],
) -> (GameOutcome, Vec<GameEvent>) {
    if !game.bet_placed() {
        game.press_bet().unwrap();
    }
    game.pull_lever().unwrap();
    let mut events = game.drain_events();
    spin_up(game);
    for (reel, index) in presses {
        aim(game, reel, index);
        game.press_stop(reel).unwrap();
    }
    events.extend(finish(game));
    (outcome_of(&events), events)
}
