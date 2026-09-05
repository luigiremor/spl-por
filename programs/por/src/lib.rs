use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

pub mod errors;
pub mod message;
pub mod state;

pub use errors::PorError;
pub use message::*;
pub use state::*;

declare_id!("6KpWoEs5mAdHGpVffKXd2JrE7PRjVa2YxQbCeRvzsa9Y");

#[program]
pub mod por {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        oracle_authority: Pubkey,
        admin: Pubkey,
        reserve_decimals: u8,
        max_value: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.mint = ctx.accounts.mint.key();
        config.oracle_authority = oracle_authority;
        config.admin = admin;
        config.schema_version = SCHEMA_VERSION;
        config.currency = CURRENCY_BRL;
        config.reserve_decimals = reserve_decimals;
        config.max_value = max_value;
        config.next_sequence = 0;
        config.log_state = ctx.accounts.log_state.key();
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = payer,
        space = PorConfig::SPACE,
        seeds = [b"config", mint.key().as_ref()],
        bump
    )]
    pub config: Account<'info, PorConfig>,
    #[account(
        init,
        payer = payer,
        space = LogState::SPACE,
        seeds = [b"log", mint.key().as_ref()],
        bump
    )]
    pub log_state: Account<'info, LogState>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
