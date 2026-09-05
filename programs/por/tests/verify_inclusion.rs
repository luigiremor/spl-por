mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_sdk::{instruction::Instruction, signature::Signer, transaction::Transaction};

fn verify_ix(
    env: &common::Env,
    sequence: u64,
    account_id: [u8; 32],
    balance: u64,
    proof: Vec<por::ProofStepArg>,
) -> Instruction {
    Instruction {
        program_id: por::ID,
        accounts: por::accounts::VerifyInclusion {
            config: env.config_pda,
            audit_record: common::audit_pda(env, sequence),
        }
        .to_account_metas(None),
        data: por::instruction::VerifyInclusion {
            account_id,
            balance,
            proof,
        }
        .data(),
    }
}

fn send_verify(
    env: &mut common::Env,
    ix: Instruction,
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    env.svm.expire_blockhash();
    let payer = env.payer.insecure_clone();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        env.svm.latest_blockhash(),
    );
    env.svm.send_transaction(tx)
}

/// 3 leaves in strictly increasing account_id order, balances in cents.
fn leaves() -> Vec<([u8; 32], u64)> {
    vec![([1u8; 32], 100), ([2u8; 32], 250), ([3u8; 32], 400)]
}

fn to_args(proof: sum_tree::InclusionProof) -> Vec<por::ProofStepArg> {
    proof
        .0
        .into_iter()
        .map(|s| por::ProofStepArg {
            sibling_hash: s.sibling_hash,
            sibling_sum: s.sibling_sum,
            sibling_is_left: s.sibling_is_left,
        })
        .collect()
}

/// Anchors an attestation whose (root, total) come from a real tree.
fn setup_with_tree() -> (common::Env, sum_tree::SumTree) {
    let mut env = common::setup(1_000_000, 6);
    let oracle = env.oracle.insecure_clone();
    let tree = sum_tree::build(&leaves(), 100_000_000_000).unwrap();
    let mut args = common::default_args(0);
    args.merkle_root = tree.root_hash;
    args.total_reservas = tree.total; // 750
    common::send_submit(&mut env, &oracle, args).unwrap();
    (env, tree)
}

#[test]
fn valid_single_path_proof_accepted() {
    let (mut env, tree) = setup_with_tree();
    let proof = sum_tree::prove(&tree, 1);
    let ix = verify_ix(&env, 0, [2u8; 32], 250, to_args(proof));
    send_verify(&mut env, ix).unwrap();
}

#[test]
fn tampered_balance_rejected() {
    let (mut env, tree) = setup_with_tree();
    let proof = sum_tree::prove(&tree, 1);
    // tampered balance: the recomputed root diverges from the anchored one
    let ix = verify_ix(&env, 0, [2u8; 32], 9_999, to_args(proof));
    let res = send_verify(&mut env, ix);
    common::assert_por_error(res, 6); // MessageMismatch = 6006 (proof does not re-bind)
}

#[test]
fn tampered_sibling_sum_rejected() {
    // Hu et al. vector: sibling subtotal altered without recomputing the hash
    let (mut env, tree) = setup_with_tree();
    let proof = sum_tree::prove(&tree, 1);
    let mut steps = to_args(proof);
    steps[0].sibling_sum += 1;
    let ix = verify_ix(&env, 0, [2u8; 32], 250, steps);
    let res = send_verify(&mut env, ix);
    common::assert_por_error(res, 6); // MessageMismatch = 6006
}
