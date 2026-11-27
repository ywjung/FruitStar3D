//! 기종 프로필: 릴 배열, 배당, 추첨표, 진행 타이밍.

use fruit_star_core::{REEL_COUNT, Symbol};

use crate::bonus::BonusKind;
use crate::error::ProfileError;
use crate::lottery::{Flag, LotteryTable};

/// 성립할 수 있는 역할(당첨 조합).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// REPLAY 세 개. 메달 지급은 없고 다음 게임을 무료로 한다.
    Replay,
    /// 포도 세 개.
    Grape,
    /// 체리 세 개.
    Cherry,
    /// 벨 세 개.
    Bell,
    /// 수박 세 개.
    Melon,
    /// 스타 세 개.
    Star,
    /// 빨간 7 세 개. 큰 보너스를 시작한다.
    BigRed,
    /// 파란 7 세 개. 큰 보너스를 시작한다.
    BigBlue,
    /// BAR 세 개. 작은 보너스를 시작한다.
    Regular,
}

impl Role {
    pub const COUNT: usize = 9;

    pub const ALL: [Role; Role::COUNT] = [
        Role::Replay,
        Role::Grape,
        Role::Cherry,
        Role::Bell,
        Role::Melon,
        Role::Star,
        Role::BigRed,
        Role::BigBlue,
        Role::Regular,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Role::Replay => "replay",
            Role::Grape => "grape",
            Role::Cherry => "cherry",
            Role::Bell => "bell",
            Role::Melon => "melon",
            Role::Star => "star",
            Role::BigRed => "big_red",
            Role::BigBlue => "big_blue",
            Role::Regular => "regular_bar",
        }
    }

    pub fn from_key(key: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|r| r.key() == key)
    }

    pub const fn ordinal(self) -> usize {
        self as usize
    }

    /// 이 역할을 구성하는 심볼.
    pub const fn symbol(self) -> Symbol {
        match self {
            Role::Replay => Symbol::Replay,
            Role::Grape => Symbol::Grape,
            Role::Cherry => Symbol::Cherry,
            Role::Bell => Symbol::Bell,
            Role::Melon => Symbol::Melon,
            Role::Star => Symbol::Star,
            Role::BigRed => Symbol::RedSeven,
            Role::BigBlue => Symbol::BlueSeven,
            Role::Regular => Symbol::Bar,
        }
    }

    /// 이 역할이 시작하는 보너스 (보너스 역할이 아니면 `None`).
    pub const fn bonus_kind(self) -> Option<BonusKind> {
        match self {
            Role::BigRed | Role::BigBlue => Some(BonusKind::Big),
            Role::Regular => Some(BonusKind::Regular),
            _ => None,
        }
    }

    /// 소역(그 게임에서만 유효한 역할)인지 여부.
    pub const fn is_small(self) -> bool {
        self.bonus_kind().is_none()
    }

    /// `symbol` 세 개가 유효 라인에 늘어섰을 때 성립하는 역할.
    pub fn for_symbol(symbol: Symbol) -> Option<Role> {
        Role::ALL.into_iter().find(|role| role.symbol() == symbol)
    }
}

/// 역할별 지급 메달 수.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayTable {
    payouts: [u32; Role::COUNT],
}

impl PayTable {
    pub const fn new(payouts: [u32; Role::COUNT]) -> Self {
        Self { payouts }
    }

    pub fn payout(&self, role: Role) -> u32 {
        self.payouts[role.ordinal()]
    }

    pub fn set(&mut self, role: Role, payout: u32) {
        self.payouts[role.ordinal()] = payout;
    }
}

impl Default for PayTable {
    fn default() -> Self {
        let mut table = Self::new([0; Role::COUNT]);
        table.set(Role::Grape, 8);
        table.set(Role::Cherry, 4);
        table.set(Role::Bell, 10);
        table.set(Role::Melon, 12);
        table.set(Role::Star, 15);
        table
    }
}

/// 보너스 규칙.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BonusRules {
    /// 큰 보너스가 끝나는 누적 지급 메달 수.
    pub big_payout_cap: u32,
    /// 작은 보너스가 끝나는 누적 지급 메달 수.
    pub regular_payout_cap: u32,
    /// 보너스 게임 중의 추첨표.
    pub lottery: LotteryTable,
    /// 보너스에 당첨된 게임 중 레버를 당기는 순간 STAR CHANCE 램프가 켜지는 비율 (0~100 %).
    /// 나머지는 그 게임의 릴이 모두 멈춘 뒤에 켜진다.
    pub lamp_lead_percent: u32,
}

impl BonusRules {
    pub fn payout_cap(&self, kind: BonusKind) -> u32 {
        match kind {
            BonusKind::Big => self.big_payout_cap,
            BonusKind::Regular => self.regular_payout_cap,
        }
    }
}

/// 게임 진행 타이밍 (초).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowTiming {
    /// 결과 표시 최소 유지 시간.
    pub result_hold_s: f64,
    /// 메달 한 장 지급에 걸리는 시간.
    pub payout_interval_s: f64,
}

impl Default for FlowTiming {
    fn default() -> Self {
        Self {
            result_hold_s: 0.25,
            payout_interval_s: 0.07,
        }
    }
}

/// 기종 프로필.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineProfile {
    pub id: String,
    pub name: String,
    /// 릴별 심볼 배열. 인덱스가 커질수록 릴 표면의 위쪽이다.
    pub strips: [Vec<Symbol>; REEL_COUNT],
    /// 릴당 심볼 수.
    pub symbol_count: usize,
    /// 한 게임의 베팅 메달 수 (MAX BET).
    pub bet: u32,
    /// 정지 버튼을 누른 위치에서 미끄러질 수 있는 최대 칸 수.
    pub max_slip: usize,
    /// 한 게임의 최대 지급 메달 수.
    pub max_payout: u32,
    pub paytable: PayTable,
    /// 일반 게임의 추첨표.
    pub lottery: LotteryTable,
    pub bonus: BonusRules,
    pub flow: FlowTiming,
}

impl MachineProfile {
    /// 내장 기본 기종 "FRUIT CLASSIC". `config/machines/fruit_classic.toml`과 같은 내용이다.
    pub fn fruit_classic() -> Self {
        use Symbol::{
            Bar, Bell, BlueSeven as B7, Cherry, Grape, Melon, RedSeven as R7, Replay, Star,
        };
        let left = vec![
            Grape, B7, Replay, Bar, Grape, Bell, Replay, R7, Grape, Melon, Replay, Star, Grape,
            Star, Replay, Cherry, Bell, Grape, Melon, Replay, Cherry,
        ];
        let center = vec![
            Star, Grape, R7, Replay, Cherry, Grape, Melon, Replay, B7, Grape, Bell, Replay, Bar,
            Grape, Cherry, Replay, Melon, Star, Grape, Bell, Replay,
        ];
        let right = vec![
            Replay, Melon, Grape, Bar, Replay, Melon, Grape, Cherry, Replay, B7, Grape, Star,
            Replay, R7, Bell, Grape, Star, Replay, Bell, Grape, Cherry,
        ];
        // 내장 표는 가중치 합이 분모 이하이고 중복이 없도록 작성되어 있다.
        let lottery = LotteryTable::new(vec![
            (Flag::Small(Role::Replay), 8_978),
            (Flag::Small(Role::Grape), 11_200),
            (Flag::Small(Role::Cherry), 1_820),
            (Flag::Small(Role::Bell), 655),
            (Flag::Small(Role::Melon), 546),
            (Flag::Small(Role::Star), 164),
            (Flag::Bonus(BonusKind::Big), 280),
            (Flag::Bonus(BonusKind::Regular), 172),
        ])
        .unwrap_or_default();
        let bonus_lottery =
            LotteryTable::new(vec![(Flag::Small(Role::Grape), 60_000)]).unwrap_or_default();
        Self {
            id: "fruit_classic".to_owned(),
            name: "FRUIT CLASSIC".to_owned(),
            strips: [left, center, right],
            symbol_count: 21,
            bet: 3,
            max_slip: 4,
            max_payout: 15,
            paytable: PayTable::default(),
            lottery,
            bonus: BonusRules {
                big_payout_cap: 240,
                regular_payout_cap: 96,
                lottery: bonus_lottery,
                lamp_lead_percent: 25,
            },
            flow: FlowTiming::default(),
        }
    }

    /// 프로필이 게임을 진행할 수 있는 값인지 검증한다.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.symbol_count == 0 {
            return Err(ProfileError::MustBePositive("symbol_count"));
        }
        if self.bet == 0 {
            return Err(ProfileError::MustBePositive("bet"));
        }
        if self.max_payout == 0 {
            return Err(ProfileError::MustBePositive("max_payout"));
        }
        if self.bonus.big_payout_cap == 0 {
            return Err(ProfileError::MustBePositive("bonus.big_payout_cap"));
        }
        if self.bonus.lamp_lead_percent > 100 {
            return Err(ProfileError::PercentOutOfRange {
                field: "bonus.lamp_lead_percent",
                value: self.bonus.lamp_lead_percent,
            });
        }
        if self.bonus.regular_payout_cap == 0 {
            return Err(ProfileError::MustBePositive("bonus.regular_payout_cap"));
        }
        if !(self.flow.result_hold_s.is_finite() && self.flow.result_hold_s >= 0.0) {
            return Err(ProfileError::MustBePositive("flow.result_hold"));
        }
        if !(self.flow.payout_interval_s.is_finite() && self.flow.payout_interval_s >= 0.0) {
            return Err(ProfileError::MustBePositive("flow.payout_interval"));
        }
        if self.max_slip >= self.symbol_count {
            return Err(ProfileError::SlipTooLarge {
                max_slip: self.max_slip,
                len: self.symbol_count,
            });
        }
        for (reel, strip) in self.strips.iter().enumerate() {
            if strip.len() != self.symbol_count {
                return Err(ProfileError::StripLength {
                    reel,
                    expected: self.symbol_count,
                    actual: strip.len(),
                });
            }
        }
        // 추첨될 수 있는 역할은 세 릴 모두에 심볼이 있어야 성립할 수 있다.
        for role in Role::ALL {
            let drawable = match role.bonus_kind() {
                Some(kind) => self.lottery.weight(Flag::Bonus(kind)) > 0,
                None => {
                    self.lottery.weight(Flag::Small(role)) > 0
                        || self.bonus.lottery.weight(Flag::Small(role)) > 0
                }
            };
            if !drawable {
                continue;
            }
            for (reel, strip) in self.strips.iter().enumerate() {
                if !strip.contains(&role.symbol()) {
                    return Err(ProfileError::MissingSymbol {
                        reel,
                        symbol: role.symbol().key(),
                    });
                }
            }
        }
        for role in Role::ALL {
            let payout = self.paytable.payout(role);
            if payout > self.max_payout {
                return Err(ProfileError::PayoutTooLarge {
                    role: role.key(),
                    payout,
                    max: self.max_payout,
                });
            }
        }
        Ok(())
    }
}
