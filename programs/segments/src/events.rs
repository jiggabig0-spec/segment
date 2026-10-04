use anchor_lang::prelude::*;

#[event]
pub struct SeriesCreated {
    pub series: Pubkey,
    pub underlying_mint: Pubkey,
    pub series_id: u16,
}

#[event]
pub struct SeriesFinalized {
    pub series: Pubkey,
    pub partial_mints: Vec<Pubkey>,
}

#[event]
pub struct SetMinted {
    pub series: Pubkey,
    pub user: Pubkey,
    pub deposited: u64,
    pub fee: u64,
    pub sets: u64,
}

#[event]
pub struct SetRedeemed {
    pub series: Pubkey,
    pub user: Pubkey,
    pub sets: u64,
    pub fee: u64,
    pub withdrawn: u64,
}

#[event]
pub struct FeeScheduled {
    pub series: Pubkey,
    pub mint_fee_bps: u16,
    pub redeem_fee_bps: u16,
    pub effective_at: i64,
}

#[event]
pub struct FeeApplied {
    pub series: Pubkey,
    pub mint_fee_bps: u16,
    pub redeem_fee_bps: u16,
}

#[event]
pub struct MintPauseChanged {
    pub series: Pubkey,
    pub paused: bool,
}
