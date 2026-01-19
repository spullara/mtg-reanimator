use crate::card::ManaCost;

/// Mana pool tracking each color and colorless mana
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaPool {
    pub white: u32,
    pub blue: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub colorless: u32,
}

impl ManaPool {
    pub fn new() -> Self {
        ManaPool {
            white: 0,
            blue: 0,
            black: 0,
            red: 0,
            green: 0,
            colorless: 0,
        }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    /// Add mana of a specific color
    pub fn add_mana(&mut self, color: char, amount: u32) {
        match color {
            'W' => self.white += amount,
            'U' => self.blue += amount,
            'B' => self.black += amount,
            'R' => self.red += amount,
            'G' => self.green += amount,
            'C' => self.colorless += amount,
            _ => {}
        }
    }

    /// Get total mana available
    pub fn total(&self) -> u32 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless
    }

    /// Check if we can pay a mana cost
    pub fn can_pay(&self, cost: &ManaCost) -> bool {
        // Check colored requirements
        if cost.white > self.white {
            return false;
        }
        if cost.blue > self.blue {
            return false;
        }
        if cost.black > self.black {
            return false;
        }
        if cost.red > self.red {
            return false;
        }
        if cost.green > self.green {
            return false;
        }
        if cost.colorless > self.colorless {
            return false;
        }

        // Check if we have enough remaining for generic
        let remaining = self.white - cost.white
            + self.blue - cost.blue
            + self.black - cost.black
            + self.red - cost.red
            + self.green - cost.green
            + self.colorless - cost.colorless;

        remaining >= cost.generic
    }

    /// Pay a mana cost from the pool
    pub fn pay(&mut self, cost: &ManaCost) -> bool {
        if !self.can_pay(cost) {
            return false;
        }

        // Pay colored costs first
        self.white -= cost.white;
        self.blue -= cost.blue;
        self.black -= cost.black;
        self.red -= cost.red;
        self.green -= cost.green;
        self.colorless -= cost.colorless;

        // Pay generic with remaining mana (prefer colorless, then excess colors)
        let mut generic_remaining = cost.generic;
        let colors = ['C', 'W', 'U', 'B', 'R', 'G'];

        for color in &colors {
            if generic_remaining == 0 {
                break;
            }

            let available = match color {
                'W' => self.white,
                'U' => self.blue,
                'B' => self.black,
                'R' => self.red,
                'G' => self.green,
                'C' => self.colorless,
                _ => 0,
            };

            let to_pay = std::cmp::min(available, generic_remaining);
            match color {
                'W' => self.white -= to_pay,
                'U' => self.blue -= to_pay,
                'B' => self.black -= to_pay,
                'R' => self.red -= to_pay,
                'G' => self.green -= to_pay,
                'C' => self.colorless -= to_pay,
                _ => {}
            }
            generic_remaining -= to_pay;
        }

        true
    }

    /// Clear the mana pool
    pub fn clear(&mut self) {
        self.white = 0;
        self.blue = 0;
        self.black = 0;
        self.red = 0;
        self.green = 0;
        self.colorless = 0;
    }
}

impl Default for ManaPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_mana() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 2);
        pool.add_mana('U', 1);
        assert_eq!(pool.white, 2);
        assert_eq!(pool.blue, 1);
        assert_eq!(pool.total(), 3);
    }

    #[test]
    fn test_can_pay_exact() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 2);
        pool.add_mana('U', 1);

        let cost = ManaCost {
            white: 2,
            blue: 1,
            ..Default::default()
        };

        assert!(pool.can_pay(&cost));
    }

    #[test]
    fn test_can_pay_with_generic() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 3);

        let cost = ManaCost {
            white: 1,
            generic: 2,
            ..Default::default()
        };

        assert!(pool.can_pay(&cost));
    }

    #[test]
    fn test_cannot_pay_insufficient() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 1);

        let cost = ManaCost {
            white: 2,
            ..Default::default()
        };

        assert!(!pool.can_pay(&cost));
    }

    #[test]
    fn test_pay_mana() {
        let mut pool = ManaPool::new();
        pool.add_mana('W', 3);
        pool.add_mana('U', 1);

        let cost = ManaCost {
            white: 1,
            generic: 2,
            ..Default::default()
        };

        assert!(pool.pay(&cost));
        assert_eq!(pool.white, 0);
        assert_eq!(pool.blue, 1);
    }
}

