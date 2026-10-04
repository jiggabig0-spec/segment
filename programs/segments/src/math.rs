use crate::constants::BPS_DENOMINATOR;

/// Fee on `amount`, rounded up so a non-zero rate never charges zero.
pub fn fee_ceil(amount: u64, bps: u16) -> Option<u64> {
    if bps == 0 || amount == 0 {
        return Some(0);
    }
    let num = (amount as u128).checked_mul(bps as u128)?;
    let fee = num.div_ceil(BPS_DENOMINATOR as u128);
    u64::try_from(fee).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_rate_is_free() {
        assert_eq!(fee_ceil(1_000_000, 0), Some(0));
    }

    #[test]
    fn rounds_up() {
        assert_eq!(fee_ceil(1, 30), Some(1));
        assert_eq!(fee_ceil(10_000, 30), Some(30));
        assert_eq!(fee_ceil(10_001, 30), Some(31));
    }

    #[test]
    fn never_exceeds_amount() {
        for amount in [1u64, 2, 99, 333, 10_000, u64::MAX] {
            let fee = fee_ceil(amount, 30).unwrap();
            assert!(fee <= amount);
        }
    }
}
