use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use anchor_spl::token_interface::Mint;

pub mod errors;
pub mod introspection;
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

    pub fn submit_attestation(
        ctx: Context<SubmitAttestation>,
        args: SubmitAttestationArgs,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let mint = &ctx.accounts.mint;

        // structural guards
        require_keys_eq!(mint.key(), config.mint, PorError::MintMismatch);
        require!(
            args.schema_version == config.schema_version,
            PorError::MessageMismatch
        );
        require!(
            args.sequence == config.next_sequence,
            PorError::SequenceMismatch
        );
        require!(args.decimals == mint.decimals, PorError::DecimalsMismatch);

        // conditions (a)(b)(c) of §4.7.1 — authentication by introspection
        let expected = message::build_message(config, mint.decimals, &args);
        let signer_pubkey = introspection::verify_ed25519_introspection(
            &ctx.accounts.instructions_sysvar,
            &config.oracle_authority,
            &expected,
        )?;

        // coverage reconciliation (§4.6): the program reads the supply ITSELF
        // at the execution slot; checked arithmetic (overflow aborts)
        let supply = mint.supply;
        let exponent = args
            .decimals
            .checked_sub(config.reserve_decimals)
            .ok_or(PorError::Overflow)?;
        let factor = 10u64
            .checked_pow(u32::from(exponent))
            .ok_or(PorError::Overflow)?;
        let r_norm = args
            .total_reservas
            .checked_mul(factor)
            .ok_or(PorError::Overflow)?;
        // NEVER strict equality: over-collateralization is compliant
        let status = if r_norm >= supply {
            STATUS_OK
        } else {
            STATUS_DIVERGENTE
        };

        // deterministic MMR append computed BY the program; the interface
        // carries no peaks from the caller (there is no such field in args).
        // Leaf = the attestation merkle_root (monograph §4.5: "the program
        // absorbs the root as a leaf"); sum_tree applies the 0x02 domain.
        let log = &mut ctx.accounts.log_state;
        let mut mmr = sum_tree::Mmr {
            peaks: log.peaks[..usize::from(log.peak_count)].to_vec(),
            leaf_count: log.leaf_count,
        };
        mmr.append(&args.merkle_root);
        require!(mmr.peaks.len() <= MAX_PEAKS, PorError::LogFull);
        log.peaks = [[0u8; 32]; MAX_PEAKS];
        log.peaks[..mmr.peaks.len()].copy_from_slice(&mmr.peaks);
        log.peak_count = mmr.peaks.len() as u8;
        log.leaf_count = mmr.leaf_count;

        // AuditRecord persistence
        let clock = Clock::get()?;
        let record = &mut ctx.accounts.audit_record;
        record.sequence = args.sequence;
        record.merkle_root = args.merkle_root;
        record.total_reservas = args.total_reservas;
        record.supply_at_ingestion = supply;
        record.ingestion_slot = clock.slot;
        record.ipfs_cid = args.ipfs_cid.clone();
        record.dataset_sha256 = args.dataset_sha256;
        record.signer_pubkey = signer_pubkey;
        record.status = status;

        config.next_sequence = config
            .next_sequence
            .checked_add(1)
            .ok_or(PorError::Overflow)?;

        emit!(AttestationSubmitted {
            mint: mint.key(),
            sequence: args.sequence,
            merkle_root: args.merkle_root,
            total_reservas: args.total_reservas,
            supply_at_ingestion: supply,
            ingestion_slot: clock.slot,
            status,
            signer_pubkey,
        });
        Ok(())
    }

    pub fn set_oracle_authority(
        ctx: Context<SetOracleAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.admin.key(),
            ctx.accounts.config.admin,
            PorError::Unauthorized
        );
        ctx.accounts.config.oracle_authority = new_authority;
        Ok(())
    }

    pub fn verify_inclusion(
        ctx: Context<VerifyInclusion>,
        account_id: [u8; 32],
        balance: u64,
        proof: Vec<ProofStepArg>,
    ) -> Result<()> {
        let steps = proof
            .into_iter()
            .map(|s| sum_tree::ProofStep {
                sibling_hash: s.sibling_hash,
                sibling_sum: s.sibling_sum,
                sibling_is_left: s.sibling_is_left,
            })
            .collect();
        let record = &ctx.accounts.audit_record;
        // Same per-node binding rules as stage 1: checked sum, range
        // 0 <= v <= max_value, comparison with the anchored root and total.
        sum_tree::verify_inclusion(
            &account_id,
            balance,
            &sum_tree::InclusionProof(steps),
            &record.merkle_root,
            record.total_reservas,
            ctx.accounts.config.max_value,
        )
        // enum fixed by the interface contract: a failing proof does not
        // re-bind (mismatch) to the anchored commitment
        .map_err(|_| error!(PorError::MessageMismatch))?;
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

#[derive(Accounts)]
#[instruction(args: SubmitAttestationArgs)]
pub struct SubmitAttestation<'info> {
    #[account(mut, seeds = [b"config", mint.key().as_ref()], bump)]
    pub config: Account<'info, PorConfig>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = payer,
        space = AuditRecord::SPACE,
        seeds = [b"audit", mint.key().as_ref(), &args.sequence.to_le_bytes()],
        bump
    )]
    pub audit_record: Account<'info, AuditRecord>,
    #[account(mut, seeds = [b"log", mint.key().as_ref()], bump)]
    pub log_state: Account<'info, LogState>,
    /// Pays rent for the AuditRecord. Authentication does NOT depend on it:
    /// it rests entirely on the introspection (relayer-friendly).
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: address pinned to the Instructions sysvar; read only via *_checked APIs.
    #[account(address = sysvar::instructions::ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct AttestationSubmitted {
    pub mint: Pubkey,
    pub sequence: u64,
    pub merkle_root: [u8; 32],
    pub total_reservas: u64,
    pub supply_at_ingestion: u64,
    pub ingestion_slot: u64,
    pub status: u8,
    pub signer_pubkey: Pubkey,
}

#[derive(Accounts)]
pub struct SetOracleAuthority<'info> {
    #[account(mut, seeds = [b"config", config.mint.as_ref()], bump)]
    pub config: Account<'info, PorConfig>,
    pub admin: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ProofStepArg {
    pub sibling_hash: [u8; 32],
    pub sibling_sum: u64,
    pub sibling_is_left: bool,
}

#[derive(Accounts)]
pub struct VerifyInclusion<'info> {
    #[account(seeds = [b"config", config.mint.as_ref()], bump)]
    pub config: Account<'info, PorConfig>,
    #[account(
        seeds = [b"audit", config.mint.as_ref(), &audit_record.sequence.to_le_bytes()],
        bump
    )]
    pub audit_record: Account<'info, AuditRecord>,
}
