use super::super::ClockError;

const WORD_COUNT: usize = 5;

/// Stores nonnegative `f32` seconds as integer multiples of the smallest subnormal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TimeUnits([u64; WORD_COUNT]);

impl TimeUnits {
    pub(super) const ZERO: Self = Self([0; WORD_COUNT]);

    pub(super) fn from_seconds(seconds: f32) -> Self {
        let bits = seconds.to_bits() & 0x7fff_ffff;
        let exponent = (bits >> 23) & 0xff;
        let fraction = bits & 0x007f_ffff;

        if exponent == 0 {
            return Self::from_shifted(fraction as u64, 0);
        }

        let significand = (1 << 23) | fraction;
        Self::from_shifted(significand as u64, (exponent - 1) as usize)
    }

    pub(super) fn checked_add(self, other: Self) -> Option<Self> {
        let mut result = [0; WORD_COUNT];
        let mut carry = 0_u128;

        for (index, digit) in result.iter_mut().enumerate() {
            let sum = self.0[index] as u128 + other.0[index] as u128 + carry;
            *digit = sum as u64;
            carry = sum >> 64;
        }

        (carry == 0).then_some(Self(result))
    }

    pub(super) fn checked_sub(self, other: Self) -> Option<Self> {
        if self < other {
            return None;
        }

        let mut result = [0; WORD_COUNT];
        let mut borrow = false;

        for (index, digit) in result.iter_mut().enumerate() {
            let (without_other, borrowed_other) = self.0[index].overflowing_sub(other.0[index]);
            let (difference, borrowed_previous) = without_other.overflowing_sub(u64::from(borrow));
            *digit = difference;
            borrow = borrowed_other || borrowed_previous;
        }

        (!borrow).then_some(Self(result))
    }

    pub(super) fn checked_mul_usize(self, mut multiplier: usize) -> Option<Self> {
        let mut product = Self::ZERO;
        let mut addend = self;

        while multiplier != 0 {
            if multiplier & 1 == 1 {
                product = product.checked_add(addend)?;
            }

            multiplier >>= 1;
            if multiplier != 0 {
                addend = addend.checked_add(addend)?;
            }
        }

        Some(product)
    }

    pub(super) fn rounded_seconds(self) -> Result<f32, ClockError> {
        let Some(highest_bit) = self.highest_set_bit() else {
            return Ok(0.0);
        };

        if highest_bit < 23 {
            return Ok(f32::from_bits(self.0[0] as u32));
        }

        let shift = highest_bit - 23;
        let mut significand = self.shifted_low_word(shift) as u32;
        self.round_significand(&mut significand, shift);

        let mut exponent = highest_bit - 22;
        if significand == 1 << 24 {
            significand >>= 1;
            exponent += 1;
        }

        if exponent >= 0xff {
            return Err(ClockError::TimeOverflow);
        }

        let fraction = significand & 0x007f_ffff;
        let bits = ((exponent as u32) << 23) | fraction;
        Ok(f32::from_bits(bits))
    }

    fn from_shifted(significand: u64, shift: usize) -> Self {
        let mut words = [0; WORD_COUNT];
        let word_index = shift / 64;
        let bit_offset = shift % 64;

        words[word_index] = significand << bit_offset;
        if bit_offset != 0 && word_index + 1 < WORD_COUNT {
            words[word_index + 1] = significand >> (64 - bit_offset);
        }

        Self(words)
    }

    fn highest_set_bit(self) -> Option<usize> {
        self.0
            .iter()
            .enumerate()
            .rev()
            .find(|(_, word)| **word != 0)
            .map(|(index, word)| index * 64 + (63 - word.leading_zeros() as usize))
    }

    fn shifted_low_word(self, shift: usize) -> u64 {
        let word_index = shift / 64;
        let bit_offset = shift % 64;
        let low = self.0[word_index] >> bit_offset;

        if bit_offset == 0 || word_index + 1 == WORD_COUNT {
            return low;
        }

        low | (self.0[word_index + 1] << (64 - bit_offset))
    }

    fn round_significand(self, significand: &mut u32, shift: usize) {
        if shift == 0 {
            return;
        }

        let halfway_bit = shift - 1;
        let has_halfway_bit = self.bit_is_set(halfway_bit);
        let has_lower_bits = self.has_bits_below(halfway_bit);
        let above_halfway = has_halfway_bit && has_lower_bits;
        let tie_rounds_up = has_halfway_bit && !has_lower_bits && *significand & 1 == 1;

        if above_halfway || tie_rounds_up {
            *significand += 1;
        }
    }

    fn bit_is_set(self, bit_index: usize) -> bool {
        let word_index = bit_index / 64;
        let bit_offset = bit_index % 64;
        self.0[word_index] & (1 << bit_offset) != 0
    }

    fn has_bits_below(self, bit_index: usize) -> bool {
        let full_words = bit_index / 64;
        if self.0[..full_words].iter().any(|word| *word != 0) {
            return true;
        }

        let partial_bits = bit_index % 64;
        partial_bits != 0 && self.0[full_words] & ((1_u64 << partial_bits) - 1) != 0
    }
}

impl Ord for TimeUnits {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }
}

impl PartialOrd for TimeUnits {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
