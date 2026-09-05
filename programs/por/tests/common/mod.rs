#![allow(dead_code)]

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use solana_sdk::{
    account::Account,
    instruction::{Instruction, InstructionError},
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::{Transaction, TransactionError},
};

pub struct Env {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub oracle: Keypair,
    pub admin: Keypair,
    pub mint: Pubkey,
    pub config_pda: Pubkey,
    pub log_pda: Pubkey,
}

pub fn mint_account(supply: u64, decimals: u8) -> Account {
    let mint = spl_token::state::Mint {
        mint_authority: COption::None,
        supply,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    let mut data = vec![0u8; spl_token::state::Mint::LEN];
    spl_token::state::Mint::pack(mint, &mut data).unwrap();
    Account {
        lamports: 1_000_000_000,
        data,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
    }
}

/// Boots the SVM, loads the .so, creates a synthetic mint and sends
/// initialize_config (reserve_decimals = 2, max_value = 100_000_000_000).
pub fn setup(supply: u64, decimals: u8) -> Env {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(por::ID, "../../target/deploy/por.so")
        .expect("run `anchor build` first");
    let payer = Keypair::new();
    let oracle = Keypair::new();
    let admin = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint = Pubkey::new_unique();
    svm.set_account(mint, mint_account(supply, decimals)).unwrap();

    let (config_pda, _) = Pubkey::find_program_address(&[b"config", mint.as_ref()], &por::ID);
    let (log_pda, _) = Pubkey::find_program_address(&[b"log", mint.as_ref()], &por::ID);

    let ix = Instruction {
        program_id: por::ID,
        accounts: por::accounts::InitializeConfig {
            config: config_pda,
            log_state: log_pda,
            mint,
            payer: payer.pubkey(),
            system_program: solana_sdk::system_program::id(),
        }
        .to_account_metas(None),
        data: por::instruction::InitializeConfig {
            oracle_authority: oracle.pubkey(),
            admin: admin.pubkey(),
            reserve_decimals: 2,
            max_value: 100_000_000_000,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("initialize_config failed");

    Env {
        svm,
        payer,
        oracle,
        admin,
        mint,
        config_pda,
        log_pda,
    }
}

pub fn get_config(env: &Env) -> por::state::PorConfig {
    let acc = env.svm.get_account(&env.config_pda).unwrap();
    por::state::PorConfig::try_deserialize(&mut acc.data.as_slice()).unwrap()
}

pub fn get_log(env: &Env) -> por::state::LogState {
    let acc = env.svm.get_account(&env.log_pda).unwrap();
    por::state::LogState::try_deserialize(&mut acc.data.as_slice()).unwrap()
}

/// Asserts a custom program error: Anchor code = 6000 + variant index.
pub fn assert_por_error<T: std::fmt::Debug>(
    result: Result<T, litesvm::types::FailedTransactionMetadata>,
    variant_index: u32,
) {
    let failed = result.expect_err("expected transaction failure");
    match failed.err {
        TransactionError::InstructionError(_, InstructionError::Custom(code)) => {
            assert_eq!(code, 6000 + variant_index, "logs: {:?}", failed.meta.logs)
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

/// Payload of an Ed25519Program instruction with 1 signature and the
/// (pubkey, sig, msg) triple contained in the instruction itself (indices = u16::MAX).
pub fn ed25519_ix_data(pubkey: &[u8; 32], signature: &[u8; 64], message: &[u8]) -> Vec<u8> {
    let pk_off: u16 = 16; // header(2) + offsets(14)
    let sig_off: u16 = pk_off + 32; // 48
    let msg_off: u16 = sig_off + 64; // 112
    let mut data = vec![1u8, 0u8]; // num_signatures = 1, padding
    data.extend(sig_off.to_le_bytes());
    data.extend(u16::MAX.to_le_bytes()); // signature_instruction_index
    data.extend(pk_off.to_le_bytes());
    data.extend(u16::MAX.to_le_bytes()); // public_key_instruction_index
    data.extend(msg_off.to_le_bytes());
    data.extend((message.len() as u16).to_le_bytes());
    data.extend(u16::MAX.to_le_bytes()); // message_instruction_index
    data.extend_from_slice(pubkey);
    data.extend_from_slice(signature);
    data.extend_from_slice(message);
    data
}

/// Complete instruction #0, signed by the oracle keypair (Solana keypairs ARE Ed25519).
pub fn ed25519_ix(oracle: &Keypair, message: &[u8]) -> Instruction {
    let sig: [u8; 64] = oracle.sign_message(message).as_ref().try_into().unwrap();
    Instruction {
        program_id: solana_sdk::ed25519_program::id(),
        accounts: vec![],
        data: ed25519_ix_data(&oracle.pubkey().to_bytes(), &sig, message),
    }
}
