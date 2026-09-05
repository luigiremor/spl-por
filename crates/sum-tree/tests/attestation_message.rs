use sum_tree::AttestationMessage;

fn sample() -> AttestationMessage {
    AttestationMessage {
        schema_version: 1,
        mint: [0xAA; 32],
        sequence: 7,
        merkle_root: [0xBB; 32],
        total_reservas: 500_000,
        currency: *b"BRL",
        decimals: 6,
        reserve_decimals: 2,
        ipfs_cid: "bafytestcid".to_string(),
        dataset_sha256: [0xCC; 32],
    }
}

#[test]
fn canonical_bytes_layout_matches_contract() {
    let bytes = sample().canonical_bytes();
    let cid = b"bafytestcid";
    assert_eq!(bytes.len(), 121 + cid.len());
    assert_eq!(&bytes[0..2], &1u16.to_be_bytes());
    assert_eq!(&bytes[2..34], &[0xAA; 32]);
    assert_eq!(&bytes[34..42], &7u64.to_be_bytes());
    assert_eq!(&bytes[42..74], &[0xBB; 32]);
    assert_eq!(&bytes[74..82], &500_000u64.to_be_bytes());
    assert_eq!(&bytes[82..85], b"BRL");
    assert_eq!(bytes[85], 6);
    assert_eq!(bytes[86], 2);
    assert_eq!(&bytes[87..89], &[0x00, 0x0b]);
    assert_eq!(&bytes[89..89 + cid.len()], cid);
    assert_eq!(&bytes[89 + cid.len()..], &[0xCC; 32]);
}

#[test]
fn canonical_bytes_is_stable_across_calls_and_clones() {
    let msg = sample();
    assert_eq!(msg.canonical_bytes(), msg.canonical_bytes());
    assert_eq!(msg.canonical_bytes(), msg.clone().canonical_bytes());
}

#[test]
fn cid_length_prefix_is_u16_big_endian() {
    let mut msg = sample();
    msg.ipfs_cid = "x".repeat(300);
    let bytes = msg.canonical_bytes();
    assert_eq!(&bytes[87..89], &[0x01, 0x2c]);
    assert_eq!(bytes.len(), 121 + 300);
}
