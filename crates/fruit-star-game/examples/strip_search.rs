//! 릴 배열 검증·탐색 도구.
//!
//! 릴 제어가 다음 두 조건을 모든 누름 위치·정지 순서에서 만족하는 릴 배열을 찾는다.
//!
//! 1. 포도와 리플레이는 당첨되면 반드시 성립한다 (취소되지 않는다).
//! 2. 허용되지 않은 역할은 절대 성립하지 않으며 미끄러짐이 한도를 넘지 않는다.
//!
//! 베팅 수에 따라 유효 라인이 1·3·5줄로 달라지므로 세 경우를 모두 검사한다.
//!
//! 사용법:
//! ```text
//! cargo run --release -p fruit-star-game --example strip_search            # 내장 배열 검증
//! cargo run --release -p fruit-star-game --example strip_search -- search  # 새 배열 탐색
//! ```

use fruit_star_core::{RandomSource, SplitMix64, Symbol};
use fruit_star_game::{
    Allowed, BonusKind, MachineProfile, Role, choose_stop_lines, evaluate_lines, windows_at,
};

const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

const BONUSES: [Option<BonusKind>; 3] = [None, Some(BonusKind::Big), Some(BonusKind::Regular)];

/// 한 (허용 역할, 정지 순서)에 대해 모든 누름 위치를 진행하며 `check`를 호출한다.
/// 앞선 릴의 정지 결정을 재사용하므로 전수 탐색이 빠르다. `check`가 거짓을 돌려주면 중단한다.
fn for_each_game(
    profile: &MachineProfile,
    allowed: Allowed,
    lines: usize,
    order: [usize; 3],
    mut check: impl FnMut([usize; 3], &fruit_star_game::Evaluation, usize) -> bool,
) -> bool {
    let len = profile.symbol_count;
    let [r1, r2, r3] = order;
    for p1 in 0..len {
        let d1 = choose_stop_lines(profile, allowed, lines, [None; 3], r1, p1);
        let mut s1 = [None; 3];
        s1[r1] = Some(d1.index);
        for p2 in 0..len {
            let d2 = choose_stop_lines(profile, allowed, lines, s1, r2, p2);
            let mut s2 = s1;
            s2[r2] = Some(d2.index);
            for p3 in 0..len {
                let d3 = choose_stop_lines(profile, allowed, lines, s2, r3, p3);
                let mut stops = [0usize; 3];
                stops[r1] = d1.index;
                stops[r2] = d2.index;
                stops[r3] = d3.index;
                let mut pressed = [0usize; 3];
                pressed[r1] = p1;
                pressed[r2] = p2;
                pressed[r3] = p3;
                let result = evaluate_lines(profile, &windows_at(profile, stops), lines);
                let slip = d1.slip.max(d2.slip).max(d3.slip);
                if !check(pressed, &result, slip) {
                    return false;
                }
            }
        }
    }
    true
}

/// 조건 1: 포도·리플레이는 항상 성립한다.
fn always_pulled(profile: &MachineProfile, lines: usize) -> Result<(), String> {
    for role in [Role::Grape, Role::Replay] {
        for bonus in BONUSES {
            let allowed = Allowed {
                small: Some(role),
                bonus,
            };
            for order in ORDERS {
                let mut failure = None;
                let ok = for_each_game(profile, allowed, lines, order, |pressed, result, _| {
                    if result.has_role(role) {
                        true
                    } else {
                        failure = Some(pressed);
                        false
                    }
                });
                if !ok {
                    return Err(format!(
                        "라인 {lines}: {role:?} 불성립: 보너스 {bonus:?}, 순서 {order:?}, 누름 {failure:?}"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// 조건 2: 허용되지 않은 역할은 성립하지 않고 미끄러짐이 한도 안이다.
fn never_illegal(profile: &MachineProfile, lines: usize) -> Result<(), String> {
    let smalls = [
        None,
        Some(Role::Replay),
        Some(Role::Grape),
        Some(Role::Cherry),
        Some(Role::Bell),
        Some(Role::Melon),
        Some(Role::Star),
    ];
    for small in smalls {
        for bonus in BONUSES {
            let allowed = Allowed { small, bonus };
            for order in ORDERS {
                let mut failure = None;
                let ok = for_each_game(profile, allowed, lines, order, |pressed, result, slip| {
                    let legal = result.wins.iter().all(|w| allowed.permits(w.role));
                    if legal && slip <= profile.max_slip {
                        true
                    } else {
                        failure = Some((pressed, slip, result.wins.clone()));
                        false
                    }
                });
                if !ok {
                    return Err(format!(
                        "라인 {lines}: 규칙 위반: 허용 {allowed:?}, 순서 {order:?}, {failure:?}"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// 같은 심볼이 (원형으로) 이웃하지 않는지 검사한다.
fn no_adjacent_duplicates(strip: &[Symbol]) -> bool {
    (0..strip.len()).all(|i| strip[i] != strip[(i + 1) % strip.len()])
}

fn shuffle(values: &mut [Symbol], rng: &mut SplitMix64) {
    for i in (1..values.len()).rev() {
        let j = rng.below(i as u32 + 1) as usize;
        values.swap(i, j);
    }
}

/// 포도·리플레이 자리는 그대로 두고 나머지 심볼의 자리를 섞는다.
fn randomize(profile: &mut MachineProfile, rng: &mut SplitMix64) {
    for strip in &mut profile.strips {
        loop {
            let slots: Vec<usize> = (0..strip.len())
                .filter(|&i| !matches!(strip[i], Symbol::Grape | Symbol::Replay))
                .collect();
            let mut others: Vec<Symbol> = slots.iter().map(|&i| strip[i]).collect();
            shuffle(&mut others, rng);
            for (slot, symbol) in slots.iter().zip(others) {
                strip[*slot] = symbol;
            }
            if no_adjacent_duplicates(strip) {
                break;
            }
        }
    }
}

fn print_strips(profile: &MachineProfile) {
    for (reel, strip) in profile.strips.iter().enumerate() {
        let keys: Vec<String> = strip.iter().map(|s| format!("\"{}\"", s.key())).collect();
        println!("reel {reel}: [{}]", keys.join(", "));
    }
}

/// 베팅 수별 유효 라인 수 (1메달 1줄, 2메달 3줄, 3메달 5줄).
const LINE_MODES: [usize; 3] = [3, 1, 5];

/// 모든 유효 라인 수에서 조건 1을 검사한다. 실패가 가장 잦은 3줄부터 본다.
fn always_pulled_all(profile: &MachineProfile) -> Result<(), String> {
    LINE_MODES
        .into_iter()
        .try_for_each(|lines| always_pulled(profile, lines))
}

fn never_illegal_all(profile: &MachineProfile) -> Result<(), String> {
    LINE_MODES
        .into_iter()
        .try_for_each(|lines| never_illegal(profile, lines))
}

fn main() {
    let search = std::env::args().nth(1).as_deref() == Some("search");
    let mut profile = MachineProfile::fruit_classic();
    if !search {
        print_strips(&profile);
        match always_pulled_all(&profile).and_then(|()| never_illegal_all(&profile)) {
            Ok(()) => println!("검증 통과: 유효 라인 1·3·5줄 모두에서 두 조건을 만족합니다."),
            Err(message) => {
                println!("검증 실패: {message}");
                std::process::exit(1);
            }
        }
        return;
    }

    let seed = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2026);
    let mut rng = SplitMix64::new(seed);
    for attempt in 1..=20_000u32 {
        randomize(&mut profile, &mut rng);
        if always_pulled_all(&profile).is_err() {
            continue;
        }
        println!("시도 {attempt}: 조건 1 통과, 조건 2 검사 중...");
        if never_illegal_all(&profile).is_ok() {
            println!("시도 {attempt}: 두 조건을 모두 만족하는 배열을 찾았습니다.");
            print_strips(&profile);
            return;
        }
    }
    println!("조건을 만족하는 배열을 찾지 못했습니다.");
    std::process::exit(1);
}
