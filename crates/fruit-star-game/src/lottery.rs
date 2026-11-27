//! 레버 입력 시의 내부 추첨.

use fruit_star_core::RandomSource;

use crate::bonus::BonusKind;
use crate::error::ProfileError;
use crate::profile::Role;

/// 추첨 분모. 가중치는 이 값에 대한 분자다.
pub const LOTTERY_DENOMINATOR: u32 = 65_536;

/// 한 게임의 추첨 결과. 이번 게임에 성립이 허용되는 역할을 뜻한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    /// 꽝. 어떤 역할도 성립할 수 없다.
    Blank,
    /// 소역(메달 지급 또는 리플레이). 그 게임에서만 유효하다.
    Small(Role),
    /// 보너스. 해당 조합을 맞출 때까지 다음 게임으로 이월된다.
    Bonus(BonusKind),
}

impl Flag {
    pub fn key(self) -> &'static str {
        match self {
            Flag::Blank => "blank",
            Flag::Small(role) => role.key(),
            Flag::Bonus(kind) => kind.key(),
        }
    }

    /// 설정 파일 키로부터 만든다. `big`/`regular`은 보너스, 나머지는 소역 이름이다.
    pub fn from_key(key: &str) -> Option<Flag> {
        if key == "blank" {
            return Some(Flag::Blank);
        }
        if let Some(kind) = BonusKind::from_key(key) {
            return Some(Flag::Bonus(kind));
        }
        Role::from_key(key)
            .filter(|role| role.is_small())
            .map(Flag::Small)
    }
}

/// 가중치 기반 추첨표. 가중치 합이 분모보다 작으면 나머지는 꽝이다.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LotteryTable {
    entries: Vec<(Flag, u32)>,
}

impl LotteryTable {
    pub fn new(entries: Vec<(Flag, u32)>) -> Result<Self, ProfileError> {
        let mut seen: Vec<Flag> = Vec::with_capacity(entries.len());
        let mut sum = 0u64;
        for (flag, weight) in &entries {
            if seen.contains(flag) {
                return Err(ProfileError::DuplicateLotteryEntry(flag.key().to_owned()));
            }
            seen.push(*flag);
            sum += u64::from(*weight);
        }
        if sum > u64::from(LOTTERY_DENOMINATOR) {
            return Err(ProfileError::LotteryOverflow {
                sum,
                denominator: LOTTERY_DENOMINATOR,
            });
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[(Flag, u32)] {
        &self.entries
    }

    /// `flag`의 가중치 (없으면 0).
    pub fn weight(&self, flag: Flag) -> u32 {
        self.entries
            .iter()
            .find(|(f, _)| *f == flag)
            .map_or(0, |(_, w)| *w)
    }

    /// 추첨한다 (최대 베팅 기준).
    pub fn draw(&self, rng: &mut dyn RandomSource) -> Flag {
        self.draw_for_bet(rng, 1, 1)
    }

    /// `bet`메달을 건 게임의 추첨. 당첨 확률은 베팅 수에 비례한다
    /// (`max_bet`을 걸었을 때가 추첨표 그대로이고, 적게 걸면 그만큼 꽝이 늘어난다).
    /// 배당은 베팅 수와 무관하므로, 이렇게 해야 메달당 기대 환수가 같아진다.
    pub fn draw_for_bet(&self, rng: &mut dyn RandomSource, bet: u32, max_bet: u32) -> Flag {
        let bet = bet.clamp(1, max_bet.max(1));
        let range = u64::from(LOTTERY_DENOMINATOR) * u64::from(max_bet.max(1)) / u64::from(bet);
        let mut roll = rng.below(range.min(u64::from(u32::MAX)) as u32);
        if roll >= LOTTERY_DENOMINATOR {
            return Flag::Blank;
        }
        for (flag, weight) in &self.entries {
            if roll < *weight {
                return *flag;
            }
            roll -= *weight;
        }
        Flag::Blank
    }
}
