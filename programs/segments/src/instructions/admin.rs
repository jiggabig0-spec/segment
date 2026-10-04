use anchor_lang::prelude::*;
use anchor_lang::solana_program::bpf_loader_upgradeable;

use crate::{constants::*, errors::SegmentsError, state::Config};

#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + Config::INIT_SPACE, seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, Config>,
    /// Only the program's upgrade authority may initialise, so nobody can front-run the deploy.
    #[account(
        seeds = [crate::ID.as_ref()],
        bump,
        seeds::program = bpf_loader_upgradeable::ID,
        constraint = program_data.upgrade_authority_address == Some(payer.key()) @ SegmentsError::Unauthorized,
    )]
    pub program_data: Account<'info, ProgramData>,
    pub system_program: Program<'info, System>,
}

pub fn init_config(
    ctx: Context<InitConfig>,
    admin: Pubkey,
    guardian: Pubkey,
    treasury: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = admin;
    config.pending_admin = None;
    config.guardian = guardian;
    config.treasury = treasury;
    config.bump = ctx.bumps.config;
    Ok(())
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ SegmentsError::Unauthorized)]
    pub config: Account<'info, Config>,
}

pub fn propose_admin(ctx: Context<AdminOnly>, new_admin: Pubkey) -> Result<()> {
    ctx.accounts.config.pending_admin = Some(new_admin);
    Ok(())
}

pub fn set_guardian(ctx: Context<AdminOnly>, guardian: Pubkey) -> Result<()> {
    ctx.accounts.config.guardian = guardian;
    Ok(())
}

pub fn set_treasury(ctx: Context<AdminOnly>, treasury: Pubkey) -> Result<()> {
    ctx.accounts.config.treasury = treasury;
    Ok(())
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
}

pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let pending = config.pending_admin.ok_or(SegmentsError::NoPendingAdmin)?;
    require_keys_eq!(
        pending,
        ctx.accounts.new_admin.key(),
        SegmentsError::Unauthorized
    );
    config.admin = pending;
    config.pending_admin = None;
    Ok(())
}
