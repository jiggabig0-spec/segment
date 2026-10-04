//! Splits a tokenized stock into a fixed set of segment tokens ("partials") and back.
//!
//! Deposit N units of the underlying, receive N units of every partial. Burn N units of every
//! partial, receive N units of the underlying back. A series' composition is locked when it is
//! finalized, redemption can never be paused, and fees are hard-capped in code.

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("6PHe3DsZvs3JdYzELQSQ9hK6ReYqTYPc9rspXBKkf1rh");

#[program]
pub mod segments {
    use super::*;

    pub fn init_config(
        ctx: Context<InitConfig>,
        admin: Pubkey,
        guardian: Pubkey,
        treasury: Pubkey,
    ) -> Result<()> {
        instructions::admin::init_config(ctx, admin, guardian, treasury)
    }

    pub fn propose_admin(ctx: Context<AdminOnly>, new_admin: Pubkey) -> Result<()> {
        instructions::admin::propose_admin(ctx, new_admin)
    }

    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        instructions::admin::accept_admin(ctx)
    }

    pub fn set_guardian(ctx: Context<AdminOnly>, guardian: Pubkey) -> Result<()> {
        instructions::admin::set_guardian(ctx, guardian)
    }

    pub fn set_treasury(ctx: Context<AdminOnly>, treasury: Pubkey) -> Result<()> {
        instructions::admin::set_treasury(ctx, treasury)
    }

    pub fn create_series(
        ctx: Context<CreateSeries>,
        series_id: u16,
        metadata_uri: String,
    ) -> Result<()> {
        instructions::series::create_series(ctx, series_id, metadata_uri)
    }

    pub fn add_partial(
        ctx: Context<AddPartial>,
        weight_bps: u16,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        instructions::series::add_partial(ctx, weight_bps, name, symbol, uri)
    }

    pub fn finalize_series(ctx: Context<FinalizeSeries>) -> Result<()> {
        instructions::series::finalize_series(ctx)
    }

    pub fn set_series_uri(ctx: Context<SeriesAdmin>, metadata_uri: String) -> Result<()> {
        instructions::series::set_series_uri(ctx, metadata_uri)
    }

    pub fn schedule_fee(
        ctx: Context<SeriesAdmin>,
        mint_fee_bps: u16,
        redeem_fee_bps: u16,
    ) -> Result<()> {
        instructions::fees::schedule_fee(ctx, mint_fee_bps, redeem_fee_bps)
    }

    pub fn cancel_fee(ctx: Context<SeriesAdmin>) -> Result<()> {
        instructions::fees::cancel_fee(ctx)
    }

    pub fn apply_fee(ctx: Context<ApplyFee>) -> Result<()> {
        instructions::fees::apply_fee(ctx)
    }

    pub fn set_mint_paused(ctx: Context<SetMintPaused>, paused: bool) -> Result<()> {
        instructions::fees::set_mint_paused(ctx, paused)
    }

    pub fn sweep_fees(ctx: Context<SweepFees>) -> Result<()> {
        instructions::fees::sweep_fees(ctx)
    }

    pub fn mint_set<'info>(ctx: Context<'info, MintSet<'info>>, amount: u64) -> Result<()> {
        instructions::mint_redeem::mint_set(ctx, amount)
    }

    pub fn redeem_set<'info>(ctx: Context<'info, RedeemSet<'info>>, sets: u64) -> Result<()> {
        instructions::mint_redeem::redeem_set(ctx, sets)
    }
}
