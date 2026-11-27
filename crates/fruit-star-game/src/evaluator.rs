//! 정지한 릴의 심볼 조합 평가.

use fruit_star_core::reel::window_at;
use fruit_star_core::{REEL_COUNT, Symbol, VISIBLE_ROWS};

use crate::bonus::BonusKind;
use crate::profile::{MachineProfile, Role};

/// 릴별 표시창 심볼. `windows[릴][줄]`, 줄은 0=위, 1=가운데, 2=아래.
pub type Windows = [[Symbol; VISIBLE_ROWS]; REEL_COUNT];

/// 유효 라인 5개. 각 항목은 릴별 줄 번호다.
pub const PAYLINES: [[usize; REEL_COUNT]; 5] = [
    [1, 1, 1], // 가운데
    [0, 0, 0], // 위
    [2, 2, 2], // 아래
    [0, 1, 2], // 오른쪽 아래로 내려가는 대각선
    [2, 1, 0], // 오른쪽 위로 올라가는 대각선
];

/// 성립한 역할 한 건.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win {
    pub role: Role,
    /// 성립한 유효 라인 번호 ([`PAYLINES`] 인덱스).
    pub line: u8,
    /// 이 건의 배당 (게임당 상한 적용 전).
    pub payout: u32,
}

/// 한 게임의 평가 결과.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Evaluation {
    pub wins: Vec<Win>,
    /// 게임당 상한을 적용한 지급 메달 수.
    pub payout: u32,
    /// 리플레이 성립 여부.
    pub replay: bool,
    /// 이 게임으로 시작되는 보너스.
    pub bonus: Option<BonusKind>,
}

impl Evaluation {
    pub fn has_role(&self, role: Role) -> bool {
        self.wins.iter().any(|w| w.role == role)
    }
}

/// 정지 위치로부터 표시창 심볼을 구한다.
pub fn windows_at(profile: &MachineProfile, stops: [usize; REEL_COUNT]) -> Windows {
    [
        window_at(&profile.strips[0], stops[0]),
        window_at(&profile.strips[1], stops[1]),
        window_at(&profile.strips[2], stops[2]),
    ]
}

/// 베팅 수에 따른 유효 라인 수. 1메달은 가운데 한 줄, 2메달은 가로 세 줄,
/// 3메달 이상은 대각선을 포함한 다섯 줄이다 ([`PAYLINES`]의 앞에서부터).
pub const fn active_lines(bet: u32) -> usize {
    match bet {
        0 => 0,
        1 => 1,
        2 => 3,
        _ => PAYLINES.len(),
    }
}

/// 표시창 심볼만으로 성립 역할과 지급을 계산한다 (모든 유효 라인).
pub fn evaluate(profile: &MachineProfile, windows: &Windows) -> Evaluation {
    evaluate_lines(profile, windows, PAYLINES.len())
}

/// [`PAYLINES`]의 앞 `lines`개 라인만 유효하다고 보고 평가한다.
pub fn evaluate_lines(profile: &MachineProfile, windows: &Windows, lines: usize) -> Evaluation {
    let mut wins = Vec::new();

    for (line_index, line) in PAYLINES.iter().enumerate().take(lines) {
        let first = windows[0][line[0]];
        if windows[1][line[1]] != first || windows[2][line[2]] != first {
            continue;
        }
        if let Some(role) = Role::for_symbol(first) {
            wins.push(Win {
                role,
                line: line_index as u8,
                payout: profile.paytable.payout(role),
            });
        }
    }

    let total: u32 = wins.iter().map(|w| w.payout).sum();
    let replay = wins.iter().any(|w| w.role == Role::Replay);
    let bonus = wins.iter().find_map(|w| w.role.bonus_kind());
    Evaluation {
        wins,
        payout: total.min(profile.max_payout),
        replay,
        bonus,
    }
}
