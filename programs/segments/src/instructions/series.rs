use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token_2022::Token2022;
use anchor_spl::token_interface::{
    spl_token_metadata_interface::state::TokenMetadata, token_metadata_initialize, Mint,
    TokenAccount, TokenInterface, TokenMetadataInitialize,
};

use crate::{
    constants::*,
    errors::SegmentsError,
    events::*,
    state::{Config, Series},
};

#[derive(Accounts)]
#[instruction(series_id: u16)]
pub struct CreateSeries<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ SegmentsError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(mint::token_program = underlying_token_program)]
    pub underlying_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = admin,
        space = 8 + Series::INIT_SPACE,
        seeds = [SERIES_SEED, underlying_mint.key().as_ref(), &series_id.to_le_bytes()],
        bump,
    )]
    pub series: Account<'info, Series>,
    #[account(
        init,
        payer = admin,
        seeds = [VAULT_SEED, series.key().as_ref()],
        bump,
        token::mint = underlying_mint,
        token::authority = series,
        token::token_program = underlying_token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub underlying_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn create_series(
    ctx: Context<CreateSeries>,
    series_id: u16,
    metadata_uri: String,
) -> Result<()> {
    require!(
        metadata_uri.len() <= MAX_URI_LEN,
        SegmentsError::StringTooLong
    );
    let series = &mut ctx.accounts.series;
    series.underlying_mint = ctx.accounts.underlying_mint.key();
    series.underlying_token_program = ctx.accounts.underlying_token_program.key();
    series.underlying_decimals = ctx.accounts.underlying_mint.decimals;
    series.vault = ctx.accounts.vault.key();
    series.series_id = series_id;
    series.partial_count = 0;
    series.partial_mints = [Pubkey::default(); MAX_PARTIALS];
    series.weights_bps = [0; MAX_PARTIALS];
    series.finalized = false;
    series.mint_paused = false;
    series.mint_fee_bps = 0;
    series.redeem_fee_bps = 0;
    series.pending_fee = None;
    series.outstanding_sets = 0;
    series.accrued_fees = 0;
    series.metadata_uri = metadata_uri;
    series.created_at = Clock::get()?.unix_timestamp;
    series.bump = ctx.bumps.series;
    series.vault_bump = ctx.bumps.vault;

    emit!(SeriesCreated {
        series: series.key(),
        underlying_mint: series.underlying_mint,
        series_id,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct AddPartial<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ SegmentsError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [SERIES_SEED, series.underlying_mint.as_ref(), &series.series_id.to_le_bytes()],
        bump = series.bump,
        constraint = !series.finalized @ SegmentsError::SeriesFinalized,
    )]
    pub series: Account<'info, Series>,
    /// Token-2022 mint whose mint authority is the series PDA forever, with no freeze
    /// authority and no permanent delegate.
    #[account(
        init,
        payer = admin,
        seeds = [PARTIAL_SEED, series.key().as_ref(), &[series.partial_count]],
        bump,
        mint::decimals = series.underlying_decimals,
        mint::authority = series,
        mint::token_program = token_program,
        extensions::metadata_pointer::authority = series,
        extensions::metadata_pointer::metadata_address = partial_mint,
    )]
    pub partial_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn add_partial(
    ctx: Context<AddPartial>,
    weight_bps: u16,
    name: String,
    symbol: String,
    uri: String,
) -> Result<()> {
    require!(name.len() <= MAX_NAME_LEN, SegmentsError::StringTooLong);
    require!(symbol.len() <= MAX_SYMBOL_LEN, SegmentsError::StringTooLong);
    require!(uri.len() <= MAX_URI_LEN, SegmentsError::StringTooLong);
    let index = ctx.accounts.series.partial_count as usize;
    require!(index < MAX_PARTIALS, SegmentsError::TooManyPartials);

    // Token-2022 reallocs the mint for the metadata itself, but the rent must already be there.
    let metadata = TokenMetadata {
        name: name.clone(),
        symbol: symbol.clone(),
        uri: uri.clone(),
        ..Default::default()
    };
    let extra = metadata
        .tlv_size_of()
        .map_err(|_| SegmentsError::MathOverflow)?;
    let mint_info = ctx.accounts.partial_mint.to_account_info();
    let needed = Rent::get()?.minimum_balance(mint_info.data_len() + extra);
    let missing = needed.saturating_sub(mint_info.lamports());
    if missing > 0 {
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                Transfer {
                    from: ctx.accounts.admin.to_account_info(),
                    to: mint_info.clone(),
                },
            ),
            missing,
        )?;
    }

    let series = &ctx.accounts.series;
    let series_id = series.series_id.to_le_bytes();
    let seeds: &[&[u8]] = &[
        SERIES_SEED,
        series.underlying_mint.as_ref(),
        &series_id,
        &[series.bump],
    ];
    token_metadata_initialize(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            TokenMetadataInitialize {
                program_id: ctx.accounts.token_program.to_account_info(),
                metadata: mint_info.clone(),
                update_authority: series.to_account_info(),
                mint: mint_info,
                mint_authority: series.to_account_info(),
            },
            &[seeds],
        ),
        name,
        symbol,
        uri,
    )?;

    let series = &mut ctx.accounts.series;
    series.partial_mints[index] = ctx.accounts.partial_mint.key();
    series.weights_bps[index] = weight_bps;
    series.partial_count += 1;
    Ok(())
}

#[derive(Accounts)]
pub struct FinalizeSeries<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ SegmentsError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [SERIES_SEED, series.underlying_mint.as_ref(), &series.series_id.to_le_bytes()],
        bump = series.bump,
        constraint = !series.finalized @ SegmentsError::SeriesFinalized,
    )]
    pub series: Account<'info, Series>,
}

pub fn finalize_series(ctx: Context<FinalizeSeries>) -> Result<()> {
    let series = &mut ctx.accounts.series;
    require!(
        series.partial_count as usize >= MIN_PARTIALS,
        SegmentsError::TooFewPartials
    );
    let total: u32 = series.weights_bps.iter().map(|w| *w as u32).sum();
    require!(total == BPS_DENOMINATOR as u32, SegmentsError::BadWeights);
    series.finalized = true;
    emit!(SeriesFinalized {
        series: series.key(),
        partial_mints: series.partial_mints().to_vec(),
    });
    Ok(())
}

#[derive(Accounts)]
pub struct SeriesAdmin<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ SegmentsError::Unauthorized)]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [SERIES_SEED, series.underlying_mint.as_ref(), &series.series_id.to_le_bytes()],
        bump = series.bump,
    )]
    pub series: Account<'info, Series>,
}

/// Points at the off-chain methodology document. Informational only.
pub fn set_series_uri(ctx: Context<SeriesAdmin>, metadata_uri: String) -> Result<()> {
    require!(
        metadata_uri.len() <= MAX_URI_LEN,
        SegmentsError::StringTooLong
    );
    ctx.accounts.series.metadata_uri = metadata_uri;
    Ok(())
}
