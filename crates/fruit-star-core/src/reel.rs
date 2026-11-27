//! 기계식 릴 한 개의 운동 모델.
//!
//! 위치는 "스텝"(심볼 한 칸) 단위의 연속 값으로 관리한다. 위치 `p`는 표시창
//! 가운데 줄에 걸린 심볼 인덱스를 뜻하며, 회전하면 `p`가 증가한다. 심볼 배열은
//! 릴 표면에서 위쪽으로 갈수록 인덱스가 커지도록 인쇄되어 있다고 정의하므로
//! 위치가 정수 `i`일 때 표시창은 위에서부터 `i+1`, `i`, `i-1` 심볼을 보여 준다.
//!
//! 모든 갱신은 경과 시간 `dt`에 대한 구간별 해석적 적분이라 프레임률과 무관하게
//! 같은 결과를 낸다.

use std::f64::consts::TAU;

use crate::error::ReelError;
use crate::symbol::Symbol;

/// 표시창에 보이는 줄 수.
pub const VISIBLE_ROWS: usize = 3;

/// 릴 상태.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReelState {
    /// 전원 투입 후 아직 한 번도 돌지 않은 정지 상태.
    #[default]
    Idle,
    /// 정속에 도달할 때까지 가속 중.
    Accelerating,
    /// 정속 회전 중. 이 상태에서만 정지 입력을 받는다.
    Spinning,
    /// 목표 위치를 향해 감속 중.
    Decelerating,
    /// 목표 위치에 도달한 뒤의 미세 진동(관성 표현).
    Settling,
    /// 완전히 정지.
    Stopped,
}

/// 릴 운동 파라미터.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReelTiming {
    /// 정속 회전 속도 (스텝/초).
    pub spin_speed: f64,
    /// 정지에서 정속까지 걸리는 시간 (초).
    pub acceleration_s: f64,
    /// 정속에서 정지까지의 기준 감속 시간 (초).
    pub deceleration_s: f64,
    /// 정지 후 진동 시간 (초).
    pub settling_s: f64,
    /// 정지 진동의 최대 진폭 (스텝).
    pub settle_amplitude: f64,
}

impl ReelTiming {
    /// 분당 회전수와 밀리초 단위 시간으로 만든다.
    pub fn from_rpm(
        rpm: f64,
        symbol_count: usize,
        acceleration_ms: f64,
        deceleration_ms: f64,
        settling_ms: f64,
    ) -> Self {
        Self {
            spin_speed: rpm / 60.0 * symbol_count as f64,
            acceleration_s: acceleration_ms / 1000.0,
            deceleration_s: deceleration_ms / 1000.0,
            settling_s: settling_ms / 1000.0,
            settle_amplitude: 0.07,
        }
    }

    fn validate(&self) -> Result<(), ReelError> {
        if !(self.spin_speed.is_finite() && self.spin_speed > 0.0) {
            return Err(ReelError::InvalidTiming("spin_speed는 0보다 커야 합니다"));
        }
        if !(self.acceleration_s.is_finite() && self.acceleration_s > 0.0) {
            return Err(ReelError::InvalidTiming("acceleration은 0보다 커야 합니다"));
        }
        if !(self.deceleration_s.is_finite() && self.deceleration_s > 0.0) {
            return Err(ReelError::InvalidTiming("deceleration은 0보다 커야 합니다"));
        }
        if !(self.settling_s.is_finite() && self.settling_s >= 0.0) {
            return Err(ReelError::InvalidTiming("settling은 0 이상이어야 합니다"));
        }
        if !(self.settle_amplitude.is_finite() && self.settle_amplitude.abs() < 0.5) {
            return Err(ReelError::InvalidTiming(
                "settle_amplitude는 0.5 스텝 미만이어야 합니다",
            ));
        }
        Ok(())
    }
}

impl Default for ReelTiming {
    fn default() -> Self {
        Self::from_rpm(80.0, 21, 350.0, 250.0, 80.0)
    }
}

/// 한 번의 [`Reel::update`] 동안 일어난 상태 전이.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReelUpdate {
    /// 가속을 마치고 정속 회전에 들어갔다.
    pub reached_speed: bool,
    /// 목표 위치에 도달해 고정되었다 (정지음이 나는 순간).
    pub locked: bool,
    /// 정지 진동까지 끝나 완전히 멈췄다.
    pub settled: bool,
}

#[derive(Debug, Clone, Copy)]
struct Braking {
    target_index: usize,
    /// 목표까지 남은 거리 (스텝).
    remaining: f64,
    /// 제동 구간 길이 (스텝). 남은 거리가 이 값 이하가 되면 감속한다.
    brake_length: f64,
    /// 제동 감속도 (스텝/초²).
    deceleration: f64,
}

/// 기계식 릴.
#[derive(Debug, Clone)]
pub struct Reel {
    id: u8,
    symbols: Vec<Symbol>,
    timing: ReelTiming,
    state: ReelState,
    /// 가운데 줄 기준 위치 (스텝, `0..len`).
    position: f64,
    /// 현재 속도 (스텝/초).
    speed: f64,
    phase_elapsed: f64,
    braking: Option<Braking>,
    /// 정지 진동에 의한 표시용 오프셋 (스텝). 논리 위치에는 영향이 없다.
    settle_offset: f64,
}

impl Reel {
    pub fn new(id: u8, symbols: Vec<Symbol>, timing: ReelTiming) -> Result<Self, ReelError> {
        if symbols.is_empty() {
            return Err(ReelError::EmptyStrip { reel: id });
        }
        timing.validate()?;
        Ok(Self {
            id,
            symbols,
            timing,
            state: ReelState::Idle,
            position: 0.0,
            speed: 0.0,
            phase_elapsed: 0.0,
            braking: None,
            settle_offset: 0.0,
        })
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// 심볼 배열은 생성 시 비어 있지 않음이 보장된다.
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn state(&self) -> ReelState {
        self.state
    }

    pub fn timing(&self) -> &ReelTiming {
        &self.timing
    }

    /// 심볼 한 칸에 해당하는 각도 (라디안).
    pub fn step_angle(&self) -> f64 {
        TAU / self.symbols.len() as f64
    }

    /// 논리 위치 (스텝, `0..len`). 정지 진동 오프셋은 포함하지 않는다.
    pub fn position(&self) -> f64 {
        self.position
    }

    /// 표시용 위치 (스텝). 정지 진동 오프셋을 포함한다.
    pub fn display_position(&self) -> f64 {
        (self.position + self.settle_offset).rem_euclid(self.symbols.len() as f64)
    }

    /// 표시용 현재 각도 (라디안, `0..2π`).
    pub fn current_angle(&self) -> f32 {
        (self.display_position() * self.step_angle()) as f32
    }

    /// 현재 각속도 (라디안/초).
    pub fn angular_velocity(&self) -> f32 {
        (self.speed * self.step_angle()) as f32
    }

    /// 정지 목표 각도. 정지 요청 이후에만 값이 있다.
    pub fn target_angle(&self) -> Option<f32> {
        self.target_index()
            .map(|index| (index as f64 * self.step_angle()) as f32)
    }

    /// 정지 목표 인덱스. 감속 중이거나 정지한 뒤에만 값이 있다.
    pub fn target_index(&self) -> Option<usize> {
        self.braking.map(|b| b.target_index)
    }

    /// 가운데 줄에 가장 가까운 심볼 인덱스.
    pub fn current_index(&self) -> usize {
        let len = self.symbols.len();
        (self.position + 0.5).floor().rem_euclid(len as f64) as usize % len
    }

    /// 회전 방향으로 다음에 가운데 줄에 도달할 심볼 인덱스.
    /// 정확히 줄 위에 있으면 그 인덱스를 돌려준다.
    pub fn next_index(&self) -> usize {
        let len = self.symbols.len();
        self.position.ceil().rem_euclid(len as f64) as usize % len
    }

    /// 정지 위치 `index`에서 표시창에 보이는 심볼 (위, 가운데, 아래).
    pub fn window_at(&self, index: usize) -> [Symbol; VISIBLE_ROWS] {
        window_at(&self.symbols, index)
    }

    /// 현재 위치에서 표시창에 보이는 심볼 (위, 가운데, 아래).
    pub fn visible(&self) -> [Symbol; VISIBLE_ROWS] {
        self.window_at(self.current_index())
    }

    /// 회전 중(가속·정속·감속)인지 여부.
    pub fn is_moving(&self) -> bool {
        matches!(
            self.state,
            ReelState::Accelerating | ReelState::Spinning | ReelState::Decelerating
        )
    }

    /// 정지 위치가 확정되었는지 여부 (진동 중 포함).
    pub fn is_locked(&self) -> bool {
        matches!(self.state, ReelState::Settling | ReelState::Stopped)
    }

    /// 회전을 시작한다. 정지 상태에서만 가능하다.
    pub fn start(&mut self) -> Result<(), ReelError> {
        match self.state {
            ReelState::Idle | ReelState::Stopped => {
                self.state = ReelState::Accelerating;
                self.phase_elapsed = 0.0;
                self.speed = 0.0;
                self.braking = None;
                self.settle_offset = 0.0;
                Ok(())
            }
            state => Err(ReelError::CannotStart {
                reel: self.id,
                state,
            }),
        }
    }

    /// `target_index`에 멈추도록 정지를 요청한다. 정속 회전 중에만 가능하며,
    /// 이미 정지 요청을 받은 릴에 대한 중복 요청은 오류로 거부한다.
    pub fn request_stop(&mut self, target_index: usize) -> Result<(), ReelError> {
        let len = self.symbols.len();
        if self.state != ReelState::Spinning {
            return Err(ReelError::CannotStop {
                reel: self.id,
                state: self.state,
            });
        }
        if target_index >= len {
            return Err(ReelError::IndexOutOfRange {
                reel: self.id,
                index: target_index,
                len,
            });
        }
        let remaining = (target_index as f64 - self.position).rem_euclid(len as f64);
        let nominal = self.speed * self.timing.deceleration_s / 2.0;
        let brake_length = remaining.min(nominal);
        let deceleration = if brake_length > f64::EPSILON {
            self.speed * self.speed / (2.0 * brake_length)
        } else {
            f64::INFINITY
        };
        self.braking = Some(Braking {
            target_index,
            remaining,
            brake_length,
            deceleration,
        });
        self.state = ReelState::Decelerating;
        self.phase_elapsed = 0.0;
        Ok(())
    }

    /// 물리 시뮬레이션 없이 즉시 `index`에 정지시킨다 (초기화·복구·테스트용).
    pub fn force_stop_at(&mut self, index: usize) -> Result<(), ReelError> {
        let len = self.symbols.len();
        if index >= len {
            return Err(ReelError::IndexOutOfRange {
                reel: self.id,
                index,
                len,
            });
        }
        self.position = index as f64;
        self.speed = 0.0;
        self.settle_offset = 0.0;
        self.phase_elapsed = 0.0;
        self.braking = Some(Braking {
            target_index: index,
            remaining: 0.0,
            brake_length: 0.0,
            deceleration: 0.0,
        });
        self.state = ReelState::Stopped;
        Ok(())
    }

    /// `dt`초만큼 시간을 진행한다. 음수나 NaN은 0으로 취급한다.
    pub fn update(&mut self, dt: f64) -> ReelUpdate {
        let mut remaining_time = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
        let mut result = ReelUpdate::default();
        let len = self.symbols.len() as f64;

        // 상태 전이가 연쇄적으로 일어날 수 있으므로 시간을 소진할 때까지 반복한다.
        // 한 번 반복할 때마다 시간이 줄거나 상태가 전진하므로 반드시 끝난다.
        for _ in 0..8 {
            match self.state {
                ReelState::Idle | ReelState::Stopped => break,
                ReelState::Accelerating => {
                    let accel = self.timing.spin_speed / self.timing.acceleration_s;
                    let left = (self.timing.acceleration_s - self.phase_elapsed).max(0.0);
                    let t = remaining_time.min(left);
                    self.position += self.speed * t + 0.5 * accel * t * t;
                    self.speed += accel * t;
                    self.phase_elapsed += t;
                    remaining_time -= t;
                    if self.phase_elapsed >= self.timing.acceleration_s - 1e-12 {
                        self.speed = self.timing.spin_speed;
                        self.state = ReelState::Spinning;
                        self.phase_elapsed = 0.0;
                        result.reached_speed = true;
                    }
                }
                ReelState::Spinning => {
                    self.position += self.speed * remaining_time;
                    remaining_time = 0.0;
                }
                ReelState::Decelerating => {
                    let Some(mut braking) = self.braking else {
                        // 불변식 위반: 감속 중에는 항상 제동 정보가 있다.
                        self.state = ReelState::Spinning;
                        continue;
                    };
                    if braking.remaining > braking.brake_length + 1e-12 {
                        // 제동 구간에 들어가기 전까지는 정속으로 진행한다.
                        let cruise = braking.remaining - braking.brake_length;
                        let t_cruise = cruise / self.speed;
                        if remaining_time >= t_cruise {
                            self.position += cruise;
                            braking.remaining = braking.brake_length;
                            remaining_time -= t_cruise;
                        } else {
                            let d = self.speed * remaining_time;
                            self.position += d;
                            braking.remaining -= d;
                            remaining_time = 0.0;
                        }
                        self.braking = Some(braking);
                    } else {
                        let t_stop = if braking.deceleration.is_finite() {
                            self.speed / braking.deceleration
                        } else {
                            0.0
                        };
                        if remaining_time >= t_stop {
                            remaining_time -= t_stop;
                            self.position = braking.target_index as f64;
                            self.speed = 0.0;
                            braking.remaining = 0.0;
                            self.braking = Some(braking);
                            self.state = ReelState::Settling;
                            self.phase_elapsed = 0.0;
                            result.locked = true;
                        } else {
                            let t = remaining_time;
                            let d = self.speed * t - 0.5 * braking.deceleration * t * t;
                            self.position += d;
                            braking.remaining = (braking.remaining - d).max(0.0);
                            self.speed -= braking.deceleration * t;
                            self.braking = Some(braking);
                            remaining_time = 0.0;
                        }
                    }
                }
                ReelState::Settling => {
                    self.phase_elapsed += remaining_time;
                    remaining_time = 0.0;
                    if self.phase_elapsed >= self.timing.settling_s {
                        self.settle_offset = 0.0;
                        self.state = ReelState::Stopped;
                        result.settled = true;
                    } else {
                        // 감쇠 진동: 살짝 지나쳤다가 되돌아오는 관성 표현.
                        let tau = self.phase_elapsed / self.timing.settling_s;
                        let envelope = (1.0 - tau) * (-3.0 * tau).exp();
                        self.settle_offset =
                            self.timing.settle_amplitude * envelope * (TAU * 1.5 * tau).sin();
                    }
                }
            }
            self.position = self.position.rem_euclid(len);
            if remaining_time <= 0.0 {
                break;
            }
        }
        self.position = self.position.rem_euclid(len);
        result
    }
}

/// 심볼 배열 `symbols`의 정지 위치 `index`에서 보이는 심볼 (위, 가운데, 아래).
pub fn window_at(symbols: &[Symbol], index: usize) -> [Symbol; VISIBLE_ROWS] {
    let len = symbols.len();
    let index = index % len;
    [
        symbols[(index + 1) % len],
        symbols[index],
        symbols[(index + len - 1) % len],
    ]
}
