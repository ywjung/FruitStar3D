//! Tiny deterministic xorshift generator (no external `rand` dependency).

/// 32-bit xorshift PRNG. The same seed always yields the same sequence.
#[derive(Debug, Clone)]
pub struct Rng(u32);

impl Rng {
    /// Creates a generator; a zero seed is remapped because xorshift cannot leave zero.
    pub fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform value in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform value in `[-1, 1)`.
    pub fn bipolar(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            let (x, y) = (a.bipolar(), b.bipolar());
            assert_eq!(x.to_bits(), y.to_bits());
            assert!((-1.0..1.0).contains(&x));
        }
    }

    #[test]
    fn zero_seed_is_usable() {
        let mut r = Rng::new(0);
        assert_ne!(r.next_u32(), 0);
    }
}
