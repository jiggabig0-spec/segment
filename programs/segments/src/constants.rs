/// Hard cap on mint and redeem fees, in basis points. Cannot be raised without a program upgrade.
pub const MAX_FEE_BPS: u16 = 30;
/// Notice required before a fee increase takes effect.
pub const FEE_TIMELOCK_SECS: i64 = 7 * 24 * 60 * 60;
pub const MIN_PARTIALS: usize = 2;
pub const MAX_PARTIALS: usize = 8;
pub const MAX_URI_LEN: usize = 200;
pub const MAX_NAME_LEN: usize = 32;
pub const MAX_SYMBOL_LEN: usize = 10;
pub const BPS_DENOMINATOR: u64 = 10_000;

pub const CONFIG_SEED: &[u8] = b"config";
pub const SERIES_SEED: &[u8] = b"series";
pub const VAULT_SEED: &[u8] = b"vault";
pub const PARTIAL_SEED: &[u8] = b"partial";
