use anchor_lang::prelude::*;

use crate::state::PorConfig;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SubmitAttestationArgs {
    pub schema_version: u16,
    pub sequence: u64,
    pub merkle_root: [u8; 32],
    pub total_reservas: u64,
    pub decimals: u8,
    pub ipfs_cid: String,
    pub dataset_sha256: [u8; 32],
}

/// Rebuilds the canonical signed message (condition (c), monograph §4.7.1):
/// instruction args + authoritative sources (schema_version / currency /
/// reserve_decimals from PorConfig; decimals READ from the mint account;
/// mint from the config).
pub fn build_message(
    config: &PorConfig,
    mint_decimals: u8,
    args: &SubmitAttestationArgs,
) -> Vec<u8> {
    sum_tree::AttestationMessage {
        schema_version: config.schema_version,
        mint: config.mint.to_bytes(),
        sequence: args.sequence,
        merkle_root: args.merkle_root,
        total_reservas: args.total_reservas,
        currency: config.currency,
        decimals: mint_decimals,
        reserve_decimals: config.reserve_decimals,
        ipfs_cid: args.ipfs_cid.clone(),
        dataset_sha256: args.dataset_sha256,
    }
    .canonical_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PorConfig;
    use anchor_lang::prelude::Pubkey;

    #[test]
    fn build_message_equals_canonical_bytes_with_authoritative_sources() {
        let mint = Pubkey::new_unique();
        let config = PorConfig {
            mint,
            oracle_authority: Pubkey::new_unique(),
            admin: Pubkey::new_unique(),
            schema_version: 7,   // source: config (NOT args)
            currency: *b"BRL",   // source: config
            reserve_decimals: 2, // source: config
            max_value: 999,
            next_sequence: 42,
            log_state: Pubkey::new_unique(),
        };
        let args = SubmitAttestationArgs {
            schema_version: 7,
            sequence: 42,
            merkle_root: [3u8; 32],
            total_reservas: 123_456,
            decimals: 99, // deliberately IGNORED: the value read from the mint wins
            ipfs_cid: "bafy-test-cid".to_string(),
            dataset_sha256: [9u8; 32],
        };
        let mint_decimals_read = 6u8;

        let expected = sum_tree::AttestationMessage {
            schema_version: 7,
            mint: mint.to_bytes(),
            sequence: 42,
            merkle_root: [3u8; 32],
            total_reservas: 123_456,
            currency: *b"BRL",
            decimals: 6, // read, not declared
            reserve_decimals: 2,
            ipfs_cid: "bafy-test-cid".to_string(),
            dataset_sha256: [9u8; 32],
        }
        .canonical_bytes();

        assert_eq!(build_message(&config, mint_decimals_read, &args), expected);
    }
}
