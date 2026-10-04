use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{
    burn, mint_to, transfer_checked, Burn, Mint, MintTo, TokenAccount, TokenInterface,
    TransferChecked,
};

use crate::{constants::*, errors::SegmentsError, events::*, math::fee_ceil, state::Series};

#[derive(Accounts)]
pub struct MintSet<'info> {
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [SERIES_SEED, series.underlying_mint.as_ref(), &series.series_id.to_le_bytes()],
        bump = series.bump,
        has_one = vault,
        has_one = underlying_mint,
        constraint = series.finalized @ SegmentsError::SeriesNotFinalized,
    )]
    pub series: Account<'info, Series>,
    pub underlying_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    #[account(mut, token::mint = underlying_mint)]
    pub user_underlying: InterfaceAccount<'info, TokenAccount>,
    #[account(address = series.underlying_token_program)]
    pub underlying_token_program: Interface<'info, TokenInterface>,
    pub token_program: Program<'info, Token2022>,
    // remaining_accounts: for each partial in order, [partial_mint (mut), recipient token account (mut)]
}

#[derive(Accounts)]
pub struct RedeemSet<'info> {
    pub user: Signer<'info>,
    // No pause check, no admin check: a complete set is always redeemable.
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
    #[account(mut, token::mint = underlying_mint)]
    pub user_underlying: InterfaceAccount<'info, TokenAccount>,
    #[account(address = series.underlying_token_program)]
    pub underlying_token_program: Interface<'info, TokenInterface>,
    pub token_program: Program<'info, Token2022>,
    // remaining_accounts: for each partial in order, [partial_mint (mut), user token account (mut)]
}

/// Checks the remaining accounts are exactly the series' partial mints, each paired with a
/// token account for that mint, and returns the pairs.
fn partial_pairs<'info>(
    series: &Series,
    remaining: &'info [AccountInfo<'info>],
) -> Result<Vec<(&'info AccountInfo<'info>, &'info AccountInfo<'info>)>> {
    let mints = series.partial_mints();
    require!(
        remaining.len() == mints.len() * 2,
        SegmentsError::WrongPartialAccounts
    );
    let mut pairs = Vec::with_capacity(mints.len());
    for (i, expected) in mints.iter().enumerate() {
        let mint = &remaining[2 * i];
        let account = &remaining[2 * i + 1];
        require_keys_eq!(mint.key(), *expected, SegmentsError::WrongPartialMint);
        require!(
            mint.is_writable && account.is_writable,
            SegmentsError::WrongPartialAccounts
        );
        let parsed = InterfaceAccount::<TokenAccount>::try_from(account)
            .map_err(|_| SegmentsError::WrongTokenAccount)?;
        require_keys_eq!(parsed.mint, *expected, SegmentsError::WrongTokenAccount);
        pairs.push((mint, account));
    }
    Ok(pairs)
}

pub fn mint_set<'info>(ctx: Context<'info, MintSet<'info>>, amount: u64) -> Result<()> {
    require!(amount > 0, SegmentsError::ZeroAmount);
    require!(!ctx.accounts.series.mint_paused, SegmentsError::MintPaused);
    let pairs = partial_pairs(&ctx.accounts.series, ctx.remaining_accounts)?;

    // Measure what actually arrived, in case the underlying charges a transfer fee.
    let before = ctx.accounts.vault.amount;
    transfer_checked(
        CpiContext::new(
            ctx.accounts.underlying_token_program.key(),
            TransferChecked {
                from: ctx.accounts.user_underlying.to_account_info(),
                mint: ctx.accounts.underlying_mint.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.underlying_mint.decimals,
    )?;
    ctx.accounts.vault.reload()?;
    let received = ctx
        .accounts
        .vault
        .amount
        .checked_sub(before)
        .ok_or(SegmentsError::MathOverflow)?;

    let fee =
        fee_ceil(received, ctx.accounts.series.mint_fee_bps).ok_or(SegmentsError::MathOverflow)?;
    let sets = received
        .checked_sub(fee)
        .ok_or(SegmentsError::MathOverflow)?;
    require!(sets > 0, SegmentsError::AmountTooSmall);

    let series = &ctx.accounts.series;
    let series_id = series.series_id.to_le_bytes();
    let seeds: &[&[u8]] = &[
        SERIES_SEED,
        series.underlying_mint.as_ref(),
        &series_id,
        &[series.bump],
    ];
    for (mint, to) in pairs {
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                MintTo {
                    mint: mint.clone(),
                    to: to.clone(),
                    authority: series.to_account_info(),
                },
                &[seeds],
            ),
            sets,
        )?;
    }

    let series = &mut ctx.accounts.series;
    series.outstanding_sets = series
        .outstanding_sets
        .checked_add(sets)
        .ok_or(SegmentsError::MathOverflow)?;
    series.accrued_fees = series
        .accrued_fees
        .checked_add(fee)
        .ok_or(SegmentsError::MathOverflow)?;
    emit!(SetMinted {
        series: series.key(),
        user: ctx.accounts.user.key(),
        deposited: received,
        fee,
        sets,
    });
    Ok(())
}

pub fn redeem_set<'info>(ctx: Context<'info, RedeemSet<'info>>, sets: u64) -> Result<()> {
    require!(sets > 0, SegmentsError::ZeroAmount);
    let pairs = partial_pairs(&ctx.accounts.series, ctx.remaining_accounts)?;

    for (mint, from) in pairs {
        burn(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Burn {
                    mint: mint.clone(),
                    from: from.clone(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            sets,
        )?;
    }

    let fee =
        fee_ceil(sets, ctx.accounts.series.redeem_fee_bps).ok_or(SegmentsError::MathOverflow)?;
    let withdrawn = sets.checked_sub(fee).ok_or(SegmentsError::MathOverflow)?;

    let series = &ctx.accounts.series;
    let series_id = series.series_id.to_le_bytes();
    let seeds: &[&[u8]] = &[
        SERIES_SEED,
        series.underlying_mint.as_ref(),
        &series_id,
        &[series.bump],
    ];
    if withdrawn > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.underlying_token_program.key(),
                TransferChecked {
                    from: ctx.accounts.vault.to_account_info(),
                    mint: ctx.accounts.underlying_mint.to_account_info(),
                    to: ctx.accounts.user_underlying.to_account_info(),
                    authority: series.to_account_info(),
                },
                &[seeds],
            ),
            withdrawn,
            ctx.accounts.underlying_mint.decimals,
        )?;
    }

    let series = &mut ctx.accounts.series;
    series.outstanding_sets = series
        .outstanding_sets
        .checked_sub(sets)
        .ok_or(SegmentsError::MathOverflow)?;
    series.accrued_fees = series
        .accrued_fees
        .checked_add(fee)
        .ok_or(SegmentsError::MathOverflow)?;
    emit!(SetRedeemed {
        series: series.key(),
        user: ctx.accounts.user.key(),
        sets,
        fee,
        withdrawn,
    });
    Ok(())
}
