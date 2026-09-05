/// Decodes exactly 64 lowercase hex chars into a 32-byte digest.
pub fn h32(hex: &str) -> [u8; 32] {
    assert_eq!(hex.len(), 64, "expected 64 hex chars");
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("invalid hex");
    }
    out
}
