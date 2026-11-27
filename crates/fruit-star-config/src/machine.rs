//! `machines/<id>.toml`: 기종 정의 (릴 배열, 배당, 추첨표, 타이밍).

use std::collections::BTreeMap;

use fruit_star_core::{ReelTiming, Symbol};
use fruit_star_game::{BonusRules, Flag, FlowTiming, LotteryTable, MachineProfile, PayTable, Role};
use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, check_range};

const FILE: &str = "machines/*.toml";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StripsSection {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReelSection {
    /// 릴 개수. 3만 지원한다.
    pub count: u32,
    /// 릴당 심볼 수.
    pub symbol_count: u32,
    /// 표시창 줄 수. 3만 지원한다.
    pub visible_rows: u32,
    /// 정속 회전 속도 (분당 회전수).
    pub spin_rpm: f64,
    pub acceleration_ms: u32,
    pub deceleration_ms: u32,
    /// 정지 진동 시간 (밀리초).
    pub settling_ms: u32,
    /// 정지 버튼을 누른 위치에서 미끄러질 수 있는 최대 칸 수.
    pub max_slip: u32,
    pub strips: StripsSection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RulesSection {
    /// 한 게임의 베팅 메달 수 (MAX BET).
    pub bet: u32,
    /// 한 게임의 최대 지급 메달 수.
    pub max_payout: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LotterySection {
    /// 일반 게임 추첨표: 역할 이름 → 65536분의 가중치.
    pub normal: BTreeMap<String, u32>,
    /// 보너스 게임 추첨표.
    pub bonus: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BonusSection {
    pub big_payout_cap: u32,
    pub regular_payout_cap: u32,
    /// 보너스 당첨 게임 중 레버를 당길 때 램프가 켜지는 비율 (%).
    pub lamp_lead_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FlowSection {
    /// 결과 표시 최소 유지 시간 (밀리초).
    pub result_hold_ms: u32,
    /// 메달 한 장 지급 간격 (밀리초).
    pub payout_interval_ms: u32,
}

/// 조작부 애니메이션 시간 (밀리초).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnimationSection {
    /// 레버 당기기 및 복귀.
    pub lever_ms: u32,
    pub stop_button_ms: u32,
    pub bet_button_ms: u32,
    pub medal_insert_ms: u32,
}

impl Default for AnimationSection {
    fn default() -> Self {
        Self {
            lever_ms: 250,
            stop_button_ms: 120,
            bet_button_ms: 100,
            medal_insert_ms: 400,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MachineConfig {
    pub id: String,
    pub name: String,
    pub reel: ReelSection,
    pub rules: RulesSection,
    /// 역할 이름 → 지급 메달 수.
    pub payouts: BTreeMap<String, u32>,
    pub lottery: LotterySection,
    pub bonus: BonusSection,
    pub flow: FlowSection,
    pub animation: AnimationSection,
}

fn strip_keys(strip: &[Symbol]) -> Vec<String> {
    strip.iter().map(|s| s.key().to_owned()).collect()
}

fn lottery_map(table: &LotteryTable) -> BTreeMap<String, u32> {
    table
        .entries()
        .iter()
        .map(|(flag, weight)| (flag.key().to_owned(), *weight))
        .collect()
}

impl MachineConfig {
    /// 도메인 프로필과 릴 타이밍으로부터 설정을 만든다.
    pub fn from_profile(profile: &MachineProfile, timing: &ReelTiming) -> Self {
        let symbol_count = profile.symbol_count as f64;
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            reel: ReelSection {
                count: 3,
                symbol_count: profile.symbol_count as u32,
                visible_rows: 3,
                spin_rpm: (timing.spin_speed * 60.0 / symbol_count * 1000.0).round() / 1000.0,
                acceleration_ms: (timing.acceleration_s * 1000.0).round() as u32,
                deceleration_ms: (timing.deceleration_s * 1000.0).round() as u32,
                settling_ms: (timing.settling_s * 1000.0).round() as u32,
                max_slip: profile.max_slip as u32,
                strips: StripsSection {
                    left: strip_keys(&profile.strips[0]),
                    center: strip_keys(&profile.strips[1]),
                    right: strip_keys(&profile.strips[2]),
                },
            },
            rules: RulesSection {
                bet: profile.bet,
                max_payout: profile.max_payout,
            },
            payouts: Role::ALL
                .into_iter()
                .filter(|role| profile.paytable.payout(*role) > 0)
                .map(|role| (role.key().to_owned(), profile.paytable.payout(role)))
                .collect(),
            lottery: LotterySection {
                normal: lottery_map(&profile.lottery),
                bonus: lottery_map(&profile.bonus.lottery),
            },
            bonus: BonusSection {
                big_payout_cap: profile.bonus.big_payout_cap,
                regular_payout_cap: profile.bonus.regular_payout_cap,
                lamp_lead_percent: profile.bonus.lamp_lead_percent,
            },
            flow: FlowSection {
                result_hold_ms: (profile.flow.result_hold_s * 1000.0).round() as u32,
                payout_interval_ms: (profile.flow.payout_interval_s * 1000.0).round() as u32,
            },
            animation: AnimationSection::default(),
        }
    }

    fn parse_strip(name: &str, keys: &[String]) -> Result<Vec<Symbol>, ConfigError> {
        keys.iter()
            .enumerate()
            .map(|(index, key)| {
                Symbol::from_key(key).ok_or_else(|| {
                    ConfigError::invalid(
                        FILE,
                        format!("reel.strips.{name}[{index}]"),
                        format!("알 수 없는 심볼 '{key}'"),
                    )
                })
            })
            .collect()
    }

    fn parse_lottery(name: &str, map: &BTreeMap<String, u32>) -> Result<LotteryTable, ConfigError> {
        let mut entries = Vec::with_capacity(map.len());
        for (key, weight) in map {
            let flag = Flag::from_key(key).ok_or_else(|| {
                ConfigError::invalid(
                    FILE,
                    format!("lottery.{name}.{key}"),
                    "알 수 없는 추첨 항목 (소역 이름 또는 big, regular)",
                )
            })?;
            if flag == Flag::Blank {
                return Err(ConfigError::invalid(
                    FILE,
                    format!("lottery.{name}.{key}"),
                    "blank는 지정할 수 없습니다 (나머지 확률이 자동으로 꽝이 됩니다)",
                ));
            }
            entries.push((flag, *weight));
        }
        // 가중치가 큰 항목부터 두면 추첨 시 탐색이 짧다. 결과 분포는 순서와 무관하다.
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.key().cmp(b.0.key())));
        LotteryTable::new(entries)
            .map_err(|e| ConfigError::invalid(FILE, format!("lottery.{name}"), e.to_string()))
    }

    /// 값의 범위를 검증한다.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.reel.count != 3 {
            return Err(ConfigError::invalid(FILE, "reel.count", "3만 지원합니다"));
        }
        if self.reel.visible_rows != 3 {
            return Err(ConfigError::invalid(
                FILE,
                "reel.visible_rows",
                "3만 지원합니다",
            ));
        }
        check_range(FILE, "reel.symbol_count", self.reel.symbol_count, 9, 40)?;
        check_range(FILE, "reel.spin_rpm", self.reel.spin_rpm, 10.0, 200.0)?;
        check_range(
            FILE,
            "reel.acceleration_ms",
            self.reel.acceleration_ms,
            50,
            3_000,
        )?;
        check_range(
            FILE,
            "reel.deceleration_ms",
            self.reel.deceleration_ms,
            20,
            2_000,
        )?;
        check_range(FILE, "reel.settling_ms", self.reel.settling_ms, 0, 1_000)?;
        check_range(FILE, "reel.max_slip", self.reel.max_slip, 0, 8)?;
        check_range(FILE, "rules.bet", self.rules.bet, 1, 3)?;
        check_range(FILE, "rules.max_payout", self.rules.max_payout, 1, 99)?;
        check_range(
            FILE,
            "bonus.big_payout_cap",
            self.bonus.big_payout_cap,
            1,
            2_000,
        )?;
        check_range(
            FILE,
            "bonus.regular_payout_cap",
            self.bonus.regular_payout_cap,
            1,
            2_000,
        )?;
        check_range(
            FILE,
            "bonus.lamp_lead_percent",
            self.bonus.lamp_lead_percent,
            0,
            100,
        )?;
        check_range(
            FILE,
            "flow.result_hold_ms",
            self.flow.result_hold_ms,
            0,
            5_000,
        )?;
        check_range(
            FILE,
            "flow.payout_interval_ms",
            self.flow.payout_interval_ms,
            0,
            1_000,
        )?;
        check_range(
            FILE,
            "animation.lever_ms",
            self.animation.lever_ms,
            20,
            2_000,
        )?;
        check_range(
            FILE,
            "animation.stop_button_ms",
            self.animation.stop_button_ms,
            20,
            2_000,
        )?;
        check_range(
            FILE,
            "animation.bet_button_ms",
            self.animation.bet_button_ms,
            20,
            2_000,
        )?;
        check_range(
            FILE,
            "animation.medal_insert_ms",
            self.animation.medal_insert_ms,
            20,
            5_000,
        )?;
        self.to_profile().map(|_| ())
    }

    /// 도메인 프로필로 변환한다 (도메인 검증 포함).
    pub fn to_profile(&self) -> Result<MachineProfile, ConfigError> {
        let mut paytable = PayTable::new([0; Role::COUNT]);
        for (key, payout) in &self.payouts {
            let role = Role::from_key(key).ok_or_else(|| {
                ConfigError::invalid(FILE, format!("payouts.{key}"), "알 수 없는 역할 이름")
            })?;
            paytable.set(role, *payout);
        }
        let profile = MachineProfile {
            id: self.id.clone(),
            name: self.name.clone(),
            strips: [
                Self::parse_strip("left", &self.reel.strips.left)?,
                Self::parse_strip("center", &self.reel.strips.center)?,
                Self::parse_strip("right", &self.reel.strips.right)?,
            ],
            symbol_count: self.reel.symbol_count as usize,
            bet: self.rules.bet,
            max_slip: self.reel.max_slip as usize,
            max_payout: self.rules.max_payout,
            paytable,
            lottery: Self::parse_lottery("normal", &self.lottery.normal)?,
            bonus: BonusRules {
                big_payout_cap: self.bonus.big_payout_cap,
                regular_payout_cap: self.bonus.regular_payout_cap,
                lottery: Self::parse_lottery("bonus", &self.lottery.bonus)?,
                lamp_lead_percent: self.bonus.lamp_lead_percent,
            },
            flow: FlowTiming {
                result_hold_s: f64::from(self.flow.result_hold_ms) / 1000.0,
                payout_interval_s: f64::from(self.flow.payout_interval_ms) / 1000.0,
            },
        };
        profile
            .validate()
            .map_err(|e| ConfigError::invalid(FILE, "(기종 규칙)", e.to_string()))?;
        Ok(profile)
    }

    /// 릴 운동 파라미터.
    pub fn reel_timing(&self) -> ReelTiming {
        ReelTiming::from_rpm(
            self.reel.spin_rpm,
            self.reel.symbol_count as usize,
            f64::from(self.reel.acceleration_ms),
            f64::from(self.reel.deceleration_ms),
            f64::from(self.reel.settling_ms),
        )
    }
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self::from_profile(&MachineProfile::fruit_classic(), &ReelTiming::default())
    }
}

impl Default for ReelSection {
    fn default() -> Self {
        MachineConfig::default().reel
    }
}

impl Default for StripsSection {
    fn default() -> Self {
        MachineConfig::default().reel.strips
    }
}

impl Default for RulesSection {
    fn default() -> Self {
        MachineConfig::default().rules
    }
}

impl Default for LotterySection {
    fn default() -> Self {
        MachineConfig::default().lottery
    }
}

impl Default for BonusSection {
    fn default() -> Self {
        MachineConfig::default().bonus
    }
}

impl Default for FlowSection {
    fn default() -> Self {
        MachineConfig::default().flow
    }
}
