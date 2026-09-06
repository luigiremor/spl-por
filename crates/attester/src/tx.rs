use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, program_pack::Pack,
    pubkey::Pubkey, signature::read_keypair_file, signer::Signer, transaction::Transaction,
};

pub fn config_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"config", mint.as_ref()], &por::ID).0
}

pub fn audit_pda(mint: &Pubkey, sequence: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[b"audit", mint.as_ref(), &sequence.to_le_bytes()],
        &por::ID,
    )
    .0
}

pub fn log_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"log", mint.as_ref()], &por::ID).0
}

/// ix#0: native Ed25519 precompile instruction carrying ONE self-referential
/// (pubkey, signature, message) triple, as the program's introspection requires.
/// Signs with `keys::sign_message`; the SDK 2.3 keypair-taking helper is
/// deprecated, so the (message, signature, pubkey) form is used instead.
pub fn ed25519_ix(oracle: &ed25519_dalek::Keypair, message: &[u8]) -> Instruction {
    let signature = crate::keys::sign_message(oracle, message);
    solana_ed25519_program::new_ed25519_instruction_with_signature(
        message,
        &signature,
        oracle.public.as_bytes(),
    )
}

fn client(rpc_url: &str) -> RpcClient {
    RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed())
}

fn payer_from(path: &std::path::Path) -> anyhow::Result<solana_sdk::signature::Keypair> {
    read_keypair_file(path)
        .map_err(|e| anyhow::anyhow!("cannot read keypair {}: {e}", path.display()))
}

/// initialize_config: creates the PorConfig for the mint. For the PoC the
/// same local key is payer, admin and oracle_authority (documented
/// simplification; rotation is covered by stage 2 tests).
pub fn init_config(
    rpc_url: &str,
    keypair_path: &std::path::Path,
    mint: &Pubkey,
) -> anyhow::Result<()> {
    let payer = payer_from(keypair_path)?;
    let rpc = client(rpc_url);
    let accounts = por::accounts::InitializeConfig {
        config: config_pda(mint),
        log_state: log_pda(mint),
        mint: *mint,
        payer: payer.pubkey(),
        system_program: solana_sdk::system_program::ID,
    }
    .to_account_metas(None);
    let data = por::instruction::InitializeConfig {
        oracle_authority: payer.pubkey(),
        admin: payer.pubkey(),
        reserve_decimals: 2,
        max_value: crate::attestation::MAX_VALUE,
    }
    .data();
    let ix = Instruction {
        program_id: por::ID,
        accounts,
        data,
    };
    let blockhash = rpc.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], blockhash);
    let sig = rpc.send_and_confirm_transaction(&tx)?;
    println!("{{\"initialize_config\":\"{sig}\"}}");
    Ok(())
}

/// Reads PorConfig.next_sequence (used when --sequence is omitted).
pub fn next_sequence(rpc: &RpcClient, mint: &Pubkey) -> anyhow::Result<u64> {
    let data = rpc.get_account_data(&config_pda(mint))?;
    let cfg = por::state::PorConfig::try_deserialize(&mut &data[..])?;
    Ok(cfg.next_sequence)
}

/// Full attestation flow: parse dataset -> dataset_sha256 -> IPFS publish ->
/// build message -> sign canonical_bytes() -> [ix#0 Ed25519Program, ix#1
/// submit_attestation] -> RPC. Writes manifest.json and signature.json to
/// `out` (the signature travels NEXT TO the manifest, outside the object
/// covered by dataset_sha256, avoiding the circular dependency — §4.3.2).
pub fn submit(
    rpc_url: &str,
    keypair_path: &std::path::Path,
    mint: &Pubkey,
    dataset_path: &std::path::Path,
    out_dir: &std::path::Path,
    sequence: Option<u64>,
) -> anyhow::Result<()> {
    let payer = payer_from(keypair_path)?;
    // same key: payer == oracle_authority (PoC simplification)
    let oracle = crate::keys::load_solana_keypair(keypair_path)?;
    let rpc = client(rpc_url);

    let bytes = std::fs::read(dataset_path)?;
    let entries = crate::snapshot::parse_dataset(&bytes)?;
    let sha = crate::snapshot::dataset_sha256(&bytes);
    let cid = crate::ipfs::ipfs_add(dataset_path)?;

    // decimals: read from the mint account (authoritative source mirrored by
    // the program; a mismatch would abort on-chain).
    let mint_state = spl_token::state::Mint::unpack(&rpc.get_account_data(mint)?)
        .map_err(|e| anyhow::anyhow!("cannot unpack mint: {e}"))?;
    let decimals = mint_state.decimals;

    let sequence = match sequence {
        Some(s) => s,
        None => next_sequence(&rpc, mint)?,
    };
    let msg = crate::attestation::build_message(
        &entries,
        mint.to_bytes(),
        sequence,
        decimals,
        cid.clone(),
        sha,
    )?;
    let canonical = msg.canonical_bytes();

    // Advisory-only fields for the off-chain manifest:
    let issued_at =
        time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
    let manifest =
        crate::attestation::build_manifest(&msg, mint_state.supply, rpc.get_slot()?, &issued_at)?;
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("manifest.json"), &manifest)?;
    let signature = crate::keys::sign_message(&oracle, &canonical);
    std::fs::write(
        out_dir.join("signature.json"),
        format!(
            "{{\"pubkey\":\"{}\",\"signature\":\"{}\"}}",
            payer.pubkey(),
            hex::encode(signature)
        ),
    )?;

    // ix#0: native Ed25519 precompile verifies (pubkey, message, signature).
    let ix0 = ed25519_ix(&oracle, &canonical);
    // ix#1: submit_attestation with the SAME fields; the program reconstructs
    // canonical_bytes() from its arguments + authoritative sources and
    // requires byte equality with the message verified by ix#0.
    let accounts = por::accounts::SubmitAttestation {
        config: config_pda(mint),
        mint: *mint,
        audit_record: audit_pda(mint, sequence),
        log_state: log_pda(mint),
        payer: payer.pubkey(),
        instructions_sysvar: solana_sdk::sysvar::instructions::ID,
        system_program: solana_sdk::system_program::ID,
    }
    .to_account_metas(None);
    let data = por::instruction::SubmitAttestation {
        args: por::message::SubmitAttestationArgs {
            schema_version: msg.schema_version,
            sequence,
            merkle_root: msg.merkle_root,
            total_reservas: msg.total_reservas,
            decimals,
            ipfs_cid: cid.clone(),
            dataset_sha256: sha,
        },
    }
    .data();
    let ix1 = Instruction {
        program_id: por::ID,
        accounts,
        data,
    };

    let blockhash = rpc.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[ix0, ix1],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    let sig = rpc.send_and_confirm_transaction(&tx)?;
    println!(
        "{{\"signature\":\"{sig}\",\"sequence\":{sequence},\"ipfs_cid\":\"{cid}\",\"total_reservas\":\"{}\",\"dataset_sha256\":\"{}\"}}",
        msg.total_reservas,
        hex::encode(sha)
    );
    Ok(())
}

/// Fetches and prints the AuditRecord as JSON (used by the E2E assertions).
pub fn show(rpc_url: &str, mint: &Pubkey, sequence: u64) -> anyhow::Result<()> {
    let rpc = client(rpc_url);
    let data = rpc.get_account_data(&audit_pda(mint, sequence))?;
    let rec = por::state::AuditRecord::try_deserialize(&mut &data[..])?;
    let status = match rec.status {
        por::state::STATUS_OK => "OK",
        por::state::STATUS_DIVERGENTE => "DIVERGENTE",
        other => anyhow::bail!("unknown status byte: {other}"),
    };
    println!(
        "{{\"sequence\":{},\"status\":\"{status}\",\"merkle_root\":\"{}\",\"total_reservas\":\"{}\",\"supply_at_ingestion\":\"{}\",\"ingestion_slot\":{},\"ipfs_cid\":\"{}\",\"dataset_sha256\":\"{}\",\"signer_pubkey\":\"{}\"}}",
        rec.sequence,
        hex::encode(rec.merkle_root),
        rec.total_reservas,
        rec.supply_at_ingestion,
        rec.ingestion_slot,
        rec.ipfs_cid,
        hex::encode(rec.dataset_sha256),
        rec.signer_pubkey,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdas_are_deterministic_and_use_documented_seeds() {
        let mint = Pubkey::new_unique();
        let (expected_cfg, _) = Pubkey::find_program_address(&[b"config", mint.as_ref()], &por::ID);
        assert_eq!(config_pda(&mint), expected_cfg);
        let (expected_audit, _) =
            Pubkey::find_program_address(&[b"audit", mint.as_ref(), &5u64.to_le_bytes()], &por::ID);
        assert_eq!(audit_pda(&mint, 5), expected_audit);
        let (expected_log, _) = Pubkey::find_program_address(&[b"log", mint.as_ref()], &por::ID);
        assert_eq!(log_pda(&mint), expected_log);
    }

    /// The program (stage 2) authenticates via instruction introspection and
    /// requires: previous ix addressed to the native Ed25519Program,
    /// num_signatures == 1, self-referential instruction indices (u16::MAX
    /// sentinel) and offsets consistent with the payload. This test pins the
    /// ix we PRODUCE to that shape, without a validator.
    #[test]
    fn ed25519_ix_is_single_sig_self_referential_and_carries_the_message() {
        let secret = ed25519_dalek::SecretKey::from_bytes(&[7u8; 32]).unwrap();
        let public: ed25519_dalek::PublicKey = (&secret).into();
        let keypair = ed25519_dalek::Keypair { secret, public };
        let message = b"spl-por layout test".to_vec();

        let ix = ed25519_ix(&keypair, &message);
        assert_eq!(ix.program_id, solana_sdk::ed25519_program::ID);

        let d = &ix.data;
        assert_eq!(d[0], 1, "num_signatures must be 1");
        // Ed25519SignatureOffsets (7 x u16 LE) starts at byte 2:
        let u16_at = |i: usize| u16::from_le_bytes([d[i], d[i + 1]]);
        let signature_offset = u16_at(2) as usize;
        let signature_ix_index = u16_at(4);
        let public_key_offset = u16_at(6) as usize;
        let public_key_ix_index = u16_at(8);
        let message_data_offset = u16_at(10) as usize;
        let message_data_size = u16_at(12) as usize;
        let message_ix_index = u16_at(14);

        assert_eq!(signature_ix_index, u16::MAX, "self-referential sentinel");
        assert_eq!(public_key_ix_index, u16::MAX, "self-referential sentinel");
        assert_eq!(message_ix_index, u16::MAX, "self-referential sentinel");
        assert_eq!(
            &d[public_key_offset..public_key_offset + 32],
            public.as_bytes()
        );
        assert_eq!(message_data_size, message.len());
        assert_eq!(
            &d[message_data_offset..message_data_offset + message_data_size],
            &message[..]
        );
        // The 64 bytes at signature_offset must be a valid strict signature
        // over the message under the declared pubkey (offsets consistent
        // with the payload, as the program's introspection requires).
        let sig = ed25519_dalek::Signature::try_from(&d[signature_offset..signature_offset + 64])
            .unwrap();
        public.verify_strict(&message, &sig).unwrap();
        // The program additionally requires the payload verified by ix#0 to
        // equal its own reconstruction; the same check is what
        // por::introspection::check_ed25519_ix_data enforces on-chain.
        let verified = por::introspection::check_ed25519_ix_data(
            d,
            &Pubkey::from(*public.as_bytes()),
            &message,
        )
        .unwrap();
        assert_eq!(verified.to_bytes(), *public.as_bytes());
    }
}
