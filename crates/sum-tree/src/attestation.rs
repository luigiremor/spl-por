#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationMessage {
    pub schema_version: u16,
    pub mint: [u8; 32],
    pub sequence: u64,
    pub merkle_root: [u8; 32],
    pub total_reservas: u64,
    pub currency: [u8; 3],
    pub decimals: u8,
    pub reserve_decimals: u8,
    pub ipfs_cid: String,
    pub dataset_sha256: [u8; 32],
}

impl AttestationMessage {
    /// Serializes the signed payload with fixed widths, big-endian integers
    /// and contract field order.
    ///
    /// # Panics
    /// Panics if `ipfs_cid` exceeds the u16 length prefix.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        const FIXED_LEN: usize = 121;
        let cid = self.ipfs_cid.as_bytes();
        assert!(
            cid.len() <= usize::from(u16::MAX),
            "ipfs_cid longer than u16 length prefix"
        );
        let mut out = Vec::with_capacity(FIXED_LEN + cid.len());
        out.extend_from_slice(&self.schema_version.to_be_bytes());
        out.extend_from_slice(&self.mint);
        out.extend_from_slice(&self.sequence.to_be_bytes());
        out.extend_from_slice(&self.merkle_root);
        out.extend_from_slice(&self.total_reservas.to_be_bytes());
        out.extend_from_slice(&self.currency);
        out.push(self.decimals);
        out.push(self.reserve_decimals);
        out.extend_from_slice(
            &u16::try_from(cid.len())
                .expect("checked above")
                .to_be_bytes(),
        );
        out.extend_from_slice(cid);
        out.extend_from_slice(&self.dataset_sha256);
        out
    }
}
