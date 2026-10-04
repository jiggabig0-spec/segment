use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    constants::*,
    errors::SegmentsError,
    events::*,
    state::{Config, PendingFee, Series},
};

/// Fee decreases apply at once. Any increase waits `FEE_TIMELOCK_SECS` so holders can exit first.
pub fn schedule_fee(
    ctx: Context<crate::instructions::SeriesAdmin>,
    mint_fee_bps: u16,
    redeem_fee_bps: u16,
) -> Result<()> {
    require!(
        mint_fee_bps <= MAX_FEE_BPS && redeem_fee_bps <= MAX_FEE_BPS,
        SegmentsError::FeeTooHigh
    );
    let series = &mut ctx.accounts.series;
    let series_key = series.key();
    if mint_fee_bps <= series.mint_fee_bps && redeem_fee_bps <= series.redeem_fee_bps {
        series.mint_fee_bps = mint_fee_bps;
        series.redeem_fee_bps = redeem_fee_bps;
        series.pending_fee = None;
        emit!(FeeApplied {
            series: series_key,
            mint_fee_bps,
            redeem_fee_bps
        });
        return Ok(());
    }
    let effective_at = Clock::get()?
        .unix_timestamp
        .checked_add(FEE_TIMELOCK_SECS)
        .ok_or(SegmentsError::MathOverflow)?;
    series.pending_fee = Some(PendingFee {
        mint_fee_bps,
        redeem_fee_bps,
        effective_at,
    });
    emit!(FeeScheduled {
        series: series_key,
        mint_fee_bps,
        redeem_fee_bps,
        effective_at
    });
    Ok(())
}

pub fn cancel_fee(ctx: Context<crate::instructions::SeriesAdmin>) -> Result<()> {
    require!(
        ctx.accounts.series.pending_fee.is_some(),
        SegmentsError::NoPendingFee
    );
    ctx.accounts.series.pending_fee = None;
    Ok(())
}

#[derive(Accounts)]
pub struct ApplyFee<'info> {
    #[account(
        mut,
        seeds = [SERIES_SEED, series.underlying_mint.as_ref(), &series.series_id.to_le_bytes()],
        bump = series.bump,
    )]
    pub series: Account<'info, Series>,
}

/// Permissionless once the timelock has passed.
pub fn apply_fee(ctx: Context<ApplyFee>) -> Result<()> {
    let series = &mut ctx.accounts.series;
    let pending = series.pending_fee.ok_or(SegmentsError::NoPendingFee)?;
    require!(
        Clock::get()?.unix_timestamp >= pending.effective_at,
        SegmentsError::FeeTimelockActive
    );
    series.mint_fee_bps = pending.mint_fee_bps;
    series.redeem_fee_bps = pending.redeem_fee_bps;
    series.pending_fee = None;
    emit!(FeeApplied {
        series: series.key(),
        mint_fee_bps: pending.mint_fee_bps,
        redeem_fee_bps: pending.redeem_fee_bps,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct SetMintPaused<'info> {
    pub authority: Signer<'info>,
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = authority.key() == config.guardian || authority.key() == config.admin @ SegmentsError::Unauthorized,
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [SERIES_SEED, series.underlying_mint.as_ref(), &series.series_id.to_le_bytes()],
        bump = series.bump,
    )]
    pub series: Account<'info, Series>,
}

/// Pauses minting only. There is deliberately no way to pause redemption.
pub fn set_mint_paused(ctx: Context<SetMintPaused>, paused: bool) -> Result<()> {
    ctx.accounts.series.mint_paused = paused;
    emit!(MintPauseChanged {
        series: ctx.accounts.series.key(),
        paused
    });
    Ok(())
}

#[derive(Accounts)]
pub struct SweepFees<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [SERIES_SEED, series.underlying_mint.as_ref(), &series.series_id.to_le_bytes()],
        bump = series.bump,
        has_one = vault,
        has_one = underlying_mint,
    )]
    pub series: Account<'info, Series>,
    pub underlying_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = underlying_mint,
        token::authority = config.treasury,
        token::token_program = underlying_token_program,
    )]
    pub treasury_account: InterfaceAccount<'info, TokenAccount>,
    #[account(address = series.underlying_token_program)]
    pub underlying_token_program: Interface<'info, TokenInterface>,
}

/// Permissionless: moves accrued fees from the vault to the treasury.
pub fn sweep_fees(ctx: Context<SweepFees>) -> Result<()> {
    let amount = ctx.accounts.series.accrued_fees;
    if amount == 0 {
        return Ok(());
    }
    let series = &ctx.accounts.series;
    let series_id = series.series_id.to_le_bytes();
    let seeds: &[&[u8]] = &[
        SERIES_SEED,
        series.underlying_mint.as_ref(),
        &series_id,
        &[series.bump],
    ];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.underlying_token_program.key(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.underlying_mint.to_account_info(),
                to: ctx.accounts.treasury_account.to_account_info(),
                authority: series.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        ctx.accounts.underlying_mint.decimals,
    )?;
    ctx.accounts.series.accrued_fees = 0;
    Ok(())
}
