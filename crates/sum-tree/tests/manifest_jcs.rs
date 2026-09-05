use sum_tree::{hex32, Manifest};

fn sample() -> Manifest {
    Manifest {
        schema_version: 1,
        mint: "11111111111111111111111111111111".to_string(),
        sequence: "1".to_string(),
        merkle_root: "049fe1ee46b049d8184aa564b0e45a683478fdd07350fbef0e4f5a94b62efc8e".to_string(),
        total_reservas: "600".to_string(),
        currency: "BRL".to_string(),
        decimals: 6,
        reserve_decimals: 2,
        ipfs_cid: "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_string(),
        dataset_sha256: "518dddfb1cb8b8440092816d245dbb82a3569dc44d13de986d404bf14b977246"
            .to_string(),
        supply_snapshot: "6000000".to_string(),
        snapshot_slot: "123456".to_string(),
        hash_alg: "SHA-256".to_string(),
        issued_at: "2026-07-02T12:00:00Z".to_string(),
    }
}

#[test]
fn jcs_output_matches_golden_file() {
    let golden = include_str!("golden/manifest_seq1.json");
    assert_eq!(sample().to_jcs().unwrap(), golden.trim_end());
}

#[test]
fn jcs_string_has_keys_in_sorted_order() {
    let jcs = sample().to_jcs().unwrap();
    let keys = [
        "currency",
        "dataset_sha256",
        "decimals",
        "hash_alg",
        "ipfs_cid",
        "issued_at",
        "merkle_root",
        "mint",
        "reserve_decimals",
        "schema_version",
        "sequence",
        "snapshot_slot",
        "supply_snapshot",
        "total_reservas",
    ];
    let positions: Vec<usize> = keys
        .iter()
        .map(|k| {
            jcs.find(&format!("\"{k}\":"))
                .unwrap_or_else(|| panic!("missing key {k}"))
        })
        .collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(positions, sorted, "keys must appear in lexicographic order");
}

#[test]
fn u64_fields_are_json_strings_and_small_ints_are_numbers() {
    let value: serde_json::Value = serde_json::from_str(&sample().to_jcs().unwrap()).unwrap();
    let obj = value.as_object().unwrap();
    for field in [
        "sequence",
        "total_reservas",
        "supply_snapshot",
        "snapshot_slot",
    ] {
        assert!(obj[field].is_string(), "{field} must be a JSON string");
    }
    for field in ["schema_version", "decimals", "reserve_decimals"] {
        assert!(obj[field].is_number(), "{field} stays a small JSON number");
    }
}

#[test]
fn manifest_roundtrips_through_jcs() {
    let manifest = sample();
    let parsed: Manifest = serde_json::from_str(&manifest.to_jcs().unwrap()).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn hex32_encodes_lowercase() {
    assert_eq!(hex32(&[0xab; 32]), "ab".repeat(32));
    let mut bytes = [0u8; 32];
    bytes[0] = 0x01;
    bytes[31] = 0xff;
    let encoded = hex32(&bytes);
    assert_eq!(&encoded[0..2], "01");
    assert_eq!(&encoded[62..64], "ff");
    assert_eq!(encoded.len(), 64);
}
