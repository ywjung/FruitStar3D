//! 실제 플레이용 난수 공급원 (`rand` 기반).

use fruit_star_core::RandomSource;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// 운영체제 엔트로피로 시드되는 난수 공급원.
#[derive(Debug)]
pub struct SystemRandom(StdRng);

impl SystemRandom {
    /// 운영체제 엔트로피로 시드한다.
    pub fn from_os() -> Self {
        Self(rand::make_rng())
    }

    /// 고정 시드로 만든다 (재현 가능한 실행용).
    pub fn from_seed(seed: u64) -> Self {
        Self(StdRng::seed_from_u64(seed))
    }
}

impl RandomSource for SystemRandom {
    fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }
}
