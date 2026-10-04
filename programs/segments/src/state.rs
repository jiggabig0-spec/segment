use anchor_lang::prelude::*;

use crate::constants::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    /// Creates series and schedules fees. Intended to be a Squads multisig.
    pub admin: Pubkey,
    /// Set by `propose_admin`, takes over on `accept_admin`.
    pub pending_admin: Option<Pubkey>,
    /// May pause and unpause minting. Has no power over redemption.
    pub guardian: Pubkey,
    /// Owner of the token accounts that receive swept fees.
    pub treasury: Pubkey,
    pub bump: u8,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace, Default, PartialEq, Eq, Debug,
)]
pub struct PendingFee {
    pub mint_fee_bps: u16,
    pub redeem_fee_bps: u16,
    pub effective_at: i64,
}

#[account]
#[derive(InitSpace)]
pub struct Series {
    pub underlying_mint: Pubkey,
    pub underlying_token_program: Pubkey,
    pub underlying_decimals: u8,
    pub vault: Pubkey,
    pub series_id: u16,
    /// Number of partials. Fixed once `finalized` is true.
    pub partial_count: u8,
    pub partial_mints: [Pubkey; MAX_PARTIALS],
    /// Informational seed weights, sum to 10,000 once finalized. Not used in any math.
    pub weights_bps: [u16; MAX_PARTIALS],
    /// Once true, the composition can never change. There is no instruction to unset it.
    pub finalized: bool,
    pub mint_paused: bool,
    pub mint_fee_bps: u16,
    pub redeem_fee_bps: u16,
    pub pending_fee: Option<PendingFee>,
    /// Complete sets in circulation. Every partial's supply equals this.
    pub outstanding_sets: u64,
    /// Fees held in the vault, waiting for `sweep_fees`.
    pub accrued_fees: u64,
    #[max_len(MAX_URI_LEN)]
    pub metadata_uri: String,
    pub created_at: i64,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Series {
    pub fn partial_mints(&self) -> &[Pubkey] {
        &self.partial_mints[..self.partial_count as usize]
    }
}
