mod common;

use common::h32;
use sha2::{Digest, Sha256};
use sum_tree::{mmr_leaf_hash, mmr_node_hash, Mmr};

fn mmr_with(n: u8) -> Mmr {
    let mut mmr = Mmr::new();
    for i in 1..=n {
        mmr.append(&[i; 32]);
    }
    mmr
}

#[test]
fn peak_count_is_popcount_of_leaf_count() {
    let expected = [1usize, 1, 2, 1, 2, 2, 3, 1];
    for (i, want) in expected.iter().enumerate() {
        let n = (i + 1) as u8;
        let mmr = mmr_with(n);
        assert_eq!(mmr.leaf_count, u64::from(n), "n={n}");
        assert_eq!(mmr.peaks.len(), *want, "n={n}");
        assert_eq!(mmr.peaks.len(), u64::from(n).count_ones() as usize, "n={n}");
    }
}

#[test]
fn append_uses_domains_0x02_and_0x03() {
    let mmr = mmr_with(2);
    let p1: [u8; 32] = Sha256::new()
        .chain_update([0x02u8])
        .chain_update([1u8; 32])
        .finalize()
        .into();
    let p2: [u8; 32] = Sha256::new()
        .chain_update([0x02u8])
        .chain_update([2u8; 32])
        .finalize()
        .into();
    let merged: [u8; 32] = Sha256::new()
        .chain_update([0x03u8])
        .chain_update(p1)
        .chain_update(p2)
        .finalize()
        .into();
    assert_eq!(mmr.peaks, vec![merged]);
    assert_eq!(mmr_leaf_hash(&[1u8; 32]), p1);
    assert_eq!(mmr_node_hash(&p1, &p2), merged);
}

#[test]
fn peaks_after_three_appends_match_frozen_vectors() {
    let mmr = mmr_with(3);
    assert_eq!(
        mmr.peaks,
        vec![
            h32("82ba9a83470c6793242e631b61a638a2ba08b3280740e5a9ac64eb68851ea17f"),
            h32("4815060358cf19e2d5859455f9135b7384a3fccf02a4801d9c83f210107d0d9d"),
        ]
    );
}

#[test]
fn eight_appends_collapse_to_a_single_frozen_peak() {
    let mmr = mmr_with(8);
    assert_eq!(
        mmr.peaks,
        vec![h32(
            "445cacd5bd6a885e2ffadb6d2dc6ce2fcfe443a659023192367f7f822bb28ef1"
        )]
    );
    assert_eq!(
        mmr.bag_peaks(),
        h32("2f87ca4f9cb09c44087ad016655a17806db17ca3e033935534ef09e1286809c3")
    );
}

#[test]
fn bag_peaks_uses_domain_0x04_and_is_deterministic() {
    let mmr = mmr_with(3);
    let mut h = Sha256::new();
    h.update([0x04u8]);
    h.update(3u64.to_be_bytes());
    for peak in &mmr.peaks {
        h.update(peak);
    }
    let expected: [u8; 32] = h.finalize().into();
    assert_eq!(mmr.bag_peaks(), expected);
    assert_eq!(
        mmr.bag_peaks(),
        h32("d43caf006837721d884599071233d3a08bda31f81bc539c3ce861c75146f1147")
    );
    assert_eq!(mmr.bag_peaks(), mmr.bag_peaks());
}

#[test]
fn replaying_the_append_sequence_reproduces_identical_state() {
    for n in 1..=8u8 {
        let a = mmr_with(n);
        let b = mmr_with(n);
        assert_eq!(a, b, "n={n}");
        assert_eq!(a.bag_peaks(), b.bag_peaks(), "n={n}");
    }
    let mut swapped = Mmr::new();
    swapped.append(&[2u8; 32]);
    swapped.append(&[1u8; 32]);
    assert_ne!(swapped.bag_peaks(), mmr_with(2).bag_peaks());
}
