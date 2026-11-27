//! 난수 공급원 추상화.
//!
//! 도메인 로직은 이 트레이트에만 의존하므로 테스트에서는 시드가 고정된
//! 결정적 생성기를, 실제 플레이에서는 `rand` 기반 생성기를 주입할 수 있다.

/// 균등 난수 공급원.
pub trait RandomSource {
    /// 균등 분포의 32비트 난수.
    fn next_u32(&mut self) -> u32;

    /// `0..bound` 범위의 편향 없는 난수. `bound == 0`이면 0을 돌려준다.
    fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        // Lemire의 곱셈 기반 기각 샘플링.
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let m = u64::from(self.next_u32()) * u64::from(bound);
            if (m as u32) >= threshold {
                return (m >> 32) as u32;
            }
        }
    }
}

/// 시드가 고정된 결정적 생성기 (SplitMix64). 테스트와 시뮬레이션용.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl RandomSource for SplitMix64 {
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}
