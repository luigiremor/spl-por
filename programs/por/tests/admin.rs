mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

fn set_authority_ix(env: &common::Env, signer: &Keypair, new_authority: Pubkey) -> Instruction {
    Instruction {
        program_id: por::ID,
        accounts: por::accounts::SetOracleAuthority {
            config: env.config_pda,
            admin: signer.pubkey(),
        }
        .to_account_metas(None),
        data: por::instruction::SetOracleAuthority { new_authority }.data(),
    }
}

fn send_set_authority(
    env: &mut common::Env,
    signer: &Keypair,
    new_authority: Pubkey,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    env.svm.airdrop(&signer.pubkey(), 1_000_000_000).unwrap();
    env.svm.expire_blockhash();
    let ix = set_authority_ix(env, signer, new_authority);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signer.pubkey()),
        &[signer],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx)
}

#[test]
fn admin_can_rotate_oracle_authority() {
    let mut env = common::setup(1_000_000, 6);
    let admin = env.admin.insecure_clone();
    let new_oracle = Keypair::new();
    send_set_authority(&mut env, &admin, new_oracle.pubkey()).unwrap();
    assert_eq!(
        common::get_config(&env).oracle_authority,
        new_oracle.pubkey()
    );
}

#[test]
fn non_admin_rejected() {
    let mut env = common::setup(1_000_000, 6);
    let intruder = Keypair::new();
    let res = send_set_authority(&mut env, &intruder, intruder.pubkey());
    common::assert_por_error(res, 9); // Unauthorized = 6009
}

#[test]
fn historical_records_remain_verifiable_under_rotated_key() {
    let mut env = common::setup(1_000_000, 6);
    let admin = env.admin.insecure_clone();
    let old_oracle = env.oracle.insecure_clone();

    // seq 0 under the old key
    let args0 = common::default_args(0);
    let msg0 = common::expected_message(&env, &args0);
    let sig0 = old_oracle.sign_message(&msg0);
    common::send_submit(&mut env, &old_oracle, args0).unwrap();

    // rotation
    let new_oracle = Keypair::new();
    send_set_authority(&mut env, &admin, new_oracle.pubkey()).unwrap();

    // (i) the historical record keeps the key of its epoch...
    let record = common::get_record(&env, 0);
    assert_eq!(record.signer_pubkey, old_oracle.pubkey());
    // ...and the signature of that epoch still verifies under signer_pubkey
    assert!(sig0.verify(record.signer_pubkey.as_ref(), &msg0));

    // (ii) the old key can NO LONGER submit
    let res = common::send_submit(&mut env, &old_oracle, common::default_args(1));
    common::assert_por_error(res, 5); // WrongSigner = 6005

    // (iii) the new key submits
    common::send_submit(&mut env, &new_oracle, common::default_args(1)).unwrap();
    assert_eq!(
        common::get_record(&env, 1).signer_pubkey,
        new_oracle.pubkey()
    );
}
