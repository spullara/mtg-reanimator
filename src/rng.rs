use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

/// Mulberry32 PRNG - matches the TypeScript implementation exactly
/// This allows running identical games between Rust and TypeScript
#[derive(Clone)]
pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    pub fn new(seed: u32) -> Self {
        Mulberry32 { state: seed }
    }

    /// Generate next random number in [0, 1)
    /// Matches TypeScript's mulberry32 exactly
    pub fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6D2B79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        let result = t ^ (t >> 14);
        (result as f64) / 4294967296.0
    }
}

/// Seeded random number generator for reproducible simulations
/// Uses Mulberry32 to match TypeScript output exactly
#[derive(Clone)]
pub struct GameRng {
    mulberry: Mulberry32,
}

impl GameRng {
    /// Create a new GameRng with an optional seed
    /// If seed is None, generates a random seed using ChaCha8
    pub fn new(seed: Option<u64>) -> Self {
        let seed = seed.unwrap_or_else(|| {
            let mut rng = ChaCha8Rng::from_entropy();
            rng.gen::<u64>()
        });

        // Use lower 32 bits for Mulberry32 (matches TypeScript behavior)
        let mulberry = Mulberry32::new(seed as u32);
        GameRng { mulberry }
    }

    /// Generate a random number in range [0, 1)
    pub fn random(&mut self) -> f64 {
        self.mulberry.next()
    }

    /// Generate a random integer in range [0, max)
    pub fn random_range(&mut self, max: usize) -> usize {
        (self.random() * max as f64).floor() as usize
    }

    /// Fisher-Yates shuffle for a mutable slice
    /// Matches TypeScript's shuffle exactly
    pub fn shuffle<T>(&mut self, array: &mut [T]) {
        for i in (1..array.len()).rev() {
            let j = (self.random() * (i + 1) as f64).floor() as usize;
            array.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_seed_produces_same_sequence() {
        let mut rng1 = GameRng::new(Some(12345));
        let mut rng2 = GameRng::new(Some(12345));

        for _ in 0..100 {
            let v1 = rng1.random();
            let v2 = rng2.random();
            assert_eq!(v1, v2, "Same seed should produce same random sequence");
        }
    }

    #[test]
    fn test_different_seeds_produce_different_sequences() {
        let mut rng1 = GameRng::new(Some(12345));
        let mut rng2 = GameRng::new(Some(54321));

        let mut same_count = 0;
        for _ in 0..100 {
            if (rng1.random() - rng2.random()).abs() < 1e-10 {
                same_count += 1;
            }
        }
        assert!(same_count < 5, "Different seeds should produce different sequences");
    }

    #[test]
    fn test_shuffle_reproducibility() {
        let mut arr1 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut arr2 = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let mut rng1 = GameRng::new(Some(42));
        let mut rng2 = GameRng::new(Some(42));

        rng1.shuffle(&mut arr1);
        rng2.shuffle(&mut arr2);

        assert_eq!(arr1, arr2, "Same seed should produce same shuffle");
    }

    #[test]
    fn test_random_range() {
        let mut rng = GameRng::new(Some(123));
        for _ in 0..1000 {
            let val = rng.random_range(10);
            assert!(val < 10, "random_range should be in [0, max)");
        }
    }

    #[test]
    fn test_mulberry32_matches_typescript() {
        // These are the exact values produced by TypeScript's mulberry32(12345)
        let expected = [
            0.9797282677609473,
            0.3067522644996643,
            0.484205421525985,
            0.817934412509203,
            0.5094283693470061,
            0.34747186047025025,
            0.07375754183158278,
            0.7663964673411101,
            0.9968264393974096,
            0.8250224851071835,
        ];

        let mut rng = GameRng::new(Some(12345));
        for (i, &exp) in expected.iter().enumerate() {
            let actual = rng.random();
            assert!(
                (actual - exp).abs() < 1e-15,
                "Value {} mismatch: expected {}, got {}",
                i, exp, actual
            );
        }
    }
}

