use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

/// Seeded random number generator for reproducible simulations
#[derive(Clone)]
pub struct GameRng {
    rng: ChaCha8Rng,
    seed: u64,
}

impl GameRng {
    /// Create a new GameRng with an optional seed
    /// If seed is None, generates a random seed
    pub fn new(seed: Option<u64>) -> Self {
        let seed = seed.unwrap_or_else(|| {
            use rand::thread_rng;
            thread_rng().gen()
        });
        
        let rng = ChaCha8Rng::seed_from_u64(seed);
        GameRng { rng, seed }
    }

    /// Get the seed used for this RNG
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Generate a random number in range [0, 1)
    pub fn random(&mut self) -> f64 {
        self.rng.gen()
    }

    /// Generate a random integer in range [0, max)
    pub fn random_range(&mut self, max: usize) -> usize {
        self.rng.gen_range(0..max)
    }

    /// Fisher-Yates shuffle for a mutable slice
    pub fn shuffle<T>(&mut self, array: &mut [T]) {
        for i in (1..array.len()).rev() {
            let j = self.random_range(i + 1);
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
    fn test_seed_getter() {
        let seed = 999;
        let rng = GameRng::new(Some(seed));
        assert_eq!(rng.seed(), seed);
    }

    #[test]
    fn test_random_range() {
        let mut rng = GameRng::new(Some(123));
        for _ in 0..1000 {
            let val = rng.random_range(10);
            assert!(val < 10, "random_range should be in [0, max)");
        }
    }
}

