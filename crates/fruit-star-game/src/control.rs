//! 릴 제어: 정지 버튼을 누른 위치로부터 실제 정지 위치를 정한다.
//!
//! 누른 위치에서 최대 `max_slip`칸까지 미끄러질 수 있다. 그 범위 안에서
//!
//! * 허용되지 않은 역할이 성립하는 위치는 피하고,
//! * 허용된 역할은 가능한 한 끌어들이며 (소역을 보너스보다 우선한다),
//! * 그 밖에는 가장 적게 미끄러지는 위치를 고른다.
//!
//! 결과는 누른 위치와 허용 역할에 대해 결정적이다.

use fruit_star_core::REEL_COUNT;
use fruit_star_core::reel::window_at;

use crate::bonus::BonusKind;
use crate::evaluator::{PAYLINES, windows_at};
use crate::lottery::Flag;
use crate::profile::{MachineProfile, Role};

/// 이번 게임에 성립이 허용된 역할.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Allowed {
    /// 이번 게임에 당첨된 소역.
    pub small: Option<Role>,
    /// 당첨되었거나 이월 중인 보너스.
    pub bonus: Option<BonusKind>,
}

impl Allowed {
    pub const NONE: Allowed = Allowed {
        small: None,
        bonus: None,
    };

    /// 추첨 결과와 이월 중인 보너스로부터 만든다.
    pub fn from_flag(flag: Flag, carried_bonus: Option<BonusKind>) -> Self {
        match flag {
            Flag::Blank => Allowed {
                small: None,
                bonus: carried_bonus,
            },
            Flag::Small(role) => Allowed {
                small: Some(role),
                bonus: carried_bonus,
            },
            Flag::Bonus(kind) => Allowed {
                small: None,
                bonus: carried_bonus.or(Some(kind)),
            },
        }
    }

    pub fn permits(&self, role: Role) -> bool {
        match role.bonus_kind() {
            Some(kind) => self.bonus == Some(kind),
            None => self.small == Some(role),
        }
    }
}

/// 정지 위치 결정 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopDecision {
    /// 실제로 멈출 인덱스.
    pub index: usize,
    /// 누른 위치에서 미끄러진 칸 수.
    pub slip: usize,
}

type Stops = [Option<usize>; REEL_COUNT];

/// 아직 멈추지 않은 릴이 있어도 판정할 수 있는 범위에서, 허용되지 않은 역할이
/// 이미 성립했는지 검사한다.
fn is_legal(profile: &MachineProfile, allowed: Allowed, lines: usize, stops: Stops) -> bool {
    if let [Some(a), Some(b), Some(c)] = stops {
        // 평가기와 같은 규칙을 할당 없이 검사한다 (전수 탐색에서 매우 자주 호출된다).
        let windows = windows_at(profile, [a, b, c]);
        for line in PAYLINES.iter().take(lines) {
            let first = windows[0][line[0]];
            if windows[1][line[1]] != first || windows[2][line[2]] != first {
                continue;
            }
            if let Some(role) = Role::for_symbol(first)
                && !allowed.permits(role)
            {
                return false;
            }
        }
    }
    true
}

/// 마지막 한 릴만 남았을 때, 그 릴을 어디서 누르더라도 미끄러짐 범위 안에
/// 합법적인 정지 위치가 남는지 검사한다.
fn keeps_last_reel_safe(
    profile: &MachineProfile,
    allowed: Allowed,
    lines: usize,
    stops: Stops,
) -> bool {
    let mut remaining = (0..REEL_COUNT).filter(|&r| stops[r].is_none());
    let (Some(last), None) = (remaining.next(), remaining.next()) else {
        return true;
    };
    let len = profile.strips[last].len();
    (0..len).all(|pressed| {
        (0..=profile.max_slip).any(|slip| {
            let mut next = stops;
            next[last] = Some((pressed + slip) % len);
            is_legal(profile, allowed, lines, next)
        })
    })
}

/// `role`이 아직 성립 가능한 유효 라인 수 (모든 릴이 멈췄다면 성립한 라인 수).
fn progress(profile: &MachineProfile, role: Role, lines: usize, stops: Stops) -> u32 {
    let symbol = role.symbol();
    PAYLINES
        .iter()
        .take(lines)
        .filter(|line| {
            (0..REEL_COUNT).all(|reel| match stops[reel] {
                Some(index) => window_at(&profile.strips[reel], index)[line[reel]] == symbol,
                None => true,
            })
        })
        .count() as u32
}

/// `roles` 중 하나라도 `stops`에서 진척이 있는지 (모든 릴이 멈췄다면 성립했는지) 여부.
fn any_progress(profile: &MachineProfile, roles: &[Role], lines: usize, stops: Stops) -> bool {
    roles
        .iter()
        .any(|&role| progress(profile, role, lines, stops) > 0)
}

/// 마지막 한 릴만 남았을 때, 그 릴을 누를 수 있는 모든 위치 중 `roles`를
/// 합법적으로 성립시킬 수 있는 위치의 수.
fn reach(
    profile: &MachineProfile,
    allowed: Allowed,
    lines: usize,
    roles: &[Role],
    stops: Stops,
    last: usize,
) -> u32 {
    if roles.is_empty() {
        return 0;
    }
    let len = profile.strips[last].len();
    (0..len)
        .filter(|&pressed| {
            (0..=profile.max_slip).any(|slip| {
                let mut next = stops;
                next[last] = Some((pressed + slip) % len);
                is_legal(profile, allowed, lines, next) && any_progress(profile, roles, lines, next)
            })
        })
        .count() as u32
}

/// 두 릴이 남았을 때, 남은 두 릴을 어떤 순서로 어디서 누르더라도 `roles`를
/// 반드시 성립시킬 수 있는지 여부. 포도·리플레이처럼 취소되면 안 되는 역할을
/// 첫 번째 정지에서부터 보장하기 위한 두 수 앞 판정이다.
fn guaranteed(
    profile: &MachineProfile,
    allowed: Allowed,
    lines: usize,
    roles: &[Role],
    stops: Stops,
) -> bool {
    if roles.is_empty() {
        return false;
    }
    let remaining: Vec<usize> = (0..REEL_COUNT).filter(|&r| stops[r].is_none()).collect();
    let [first, second] = remaining[..] else {
        return false;
    };
    [(first, second), (second, first)]
        .into_iter()
        .all(|(next, last)| {
            let len = profile.strips[next].len();
            let full = profile.strips[last].len() as u32;
            (0..len).all(|pressed| {
                (0..=profile.max_slip).any(|slip| {
                    let mut after = stops;
                    after[next] = Some((pressed + slip) % len);
                    is_legal(profile, allowed, lines, after)
                        && reach(profile, allowed, lines, roles, after, last) == full
                })
            })
        })
}

/// 후보 정지 위치의 점수 (소역 보장, 소역, 보너스). 클수록 허용된 역할에 유리하다.
///
/// * 남은 릴이 없으면: 성립 여부.
/// * 한 릴이 남았으면: 그 릴에서 성립시킬 수 있는 누름 위치의 수.
/// * 두 릴이 남았으면: 소역의 성립이 보장되는지, 그리고 성립 가능한 유효 라인이
///   남아 있는지 여부.
fn score(
    profile: &MachineProfile,
    allowed: Allowed,
    lines: usize,
    small_roles: &[Role],
    bonus_roles: &[Role],
    stops: Stops,
) -> (bool, u32, u32) {
    let mut remaining = (0..REEL_COUNT).filter(|&r| stops[r].is_none());
    match (remaining.next(), remaining.next()) {
        (Some(last), None) => (
            false,
            reach(profile, allowed, lines, small_roles, stops, last),
            reach(profile, allowed, lines, bonus_roles, stops, last),
        ),
        (Some(_), Some(_)) => (
            guaranteed(profile, allowed, lines, small_roles, stops),
            u32::from(any_progress(profile, small_roles, lines, stops)),
            u32::from(any_progress(profile, bonus_roles, lines, stops)),
        ),
        _ => (
            false,
            u32::from(any_progress(profile, small_roles, lines, stops)),
            u32::from(any_progress(profile, bonus_roles, lines, stops)),
        ),
    }
}

/// 릴 `reel`을 `pressed` 위치에서 눌렀을 때의 정지 위치를 정한다 (모든 유효 라인).
///
/// `stopped`에는 이미 정지 위치가 확정된 릴의 인덱스를 넣는다.
pub fn choose_stop(
    profile: &MachineProfile,
    allowed: Allowed,
    stopped: Stops,
    reel: usize,
    pressed: usize,
) -> StopDecision {
    choose_stop_lines(profile, allowed, PAYLINES.len(), stopped, reel, pressed)
}

/// [`choose_stop`]과 같되, [`PAYLINES`]의 앞 `lines`개 라인만 유효하다고 본다
/// (베팅 수가 적어 일부 라인만 유효한 게임).
pub fn choose_stop_lines(
    profile: &MachineProfile,
    allowed: Allowed,
    lines: usize,
    stopped: Stops,
    reel: usize,
    pressed: usize,
) -> StopDecision {
    let len = profile.strips[reel].len();
    let pressed = pressed % len;
    let small_roles: &[Role] = match &allowed.small {
        Some(role) => std::slice::from_ref(role),
        None => &[],
    };
    let bonus_roles: &[Role] = match allowed.bonus {
        Some(BonusKind::Big) => &[Role::BigRed, Role::BigBlue],
        Some(BonusKind::Regular) => &[Role::Regular],
        None => &[],
    };

    // (안전 여부, 소역 보장, 소역 점수, 보너스 점수, 적게 미끄러질수록 큰 값)
    let mut best: Option<((bool, bool, u32, u32, usize), StopDecision)> = None;
    for slip in 0..=profile.max_slip {
        let index = (pressed + slip) % len;
        let mut next = stopped;
        next[reel] = Some(index);
        if !is_legal(profile, allowed, lines, next) {
            continue;
        }
        let safe = keeps_last_reel_safe(profile, allowed, lines, next);
        let (sure, small, bonus) = score(profile, allowed, lines, small_roles, bonus_roles, next);
        let key = (safe, sure, small, bonus, profile.max_slip - slip);
        if best.as_ref().is_none_or(|(best_key, _)| key > *best_key) {
            best = Some((key, StopDecision { index, slip }));
        }
    }
    if let Some((_, decision)) = best {
        return decision;
    }

    // 미끄러짐 범위 안에 합법적인 위치가 없으면 범위를 넘어 가장 가까운 합법 위치로 간다.
    // 내장 릴 배열에서는 일어나지 않음을 테스트로 확인한다.
    for slip in (profile.max_slip + 1)..len {
        let index = (pressed + slip) % len;
        let mut next = stopped;
        next[reel] = Some(index);
        if is_legal(profile, allowed, lines, next) {
            return StopDecision { index, slip };
        }
    }
    StopDecision {
        index: pressed,
        slip: 0,
    }
}
