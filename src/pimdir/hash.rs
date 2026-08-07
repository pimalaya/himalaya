//! Content hashing for the pimdir blob store.
//!
//! io-replica treats an [`io_replica::object::ReplicaHash`] as an opaque,
//! consumer-computed string; it never hashes anything. Bodies are
//! content-addressed so a message present in several places is stored once.
//!
//! This digest MUST match Neverest's (`neverest/src/offline/hash.rs`) and
//! himalaya-android-m3's `content_hash` — a 128-bit FNV-1a variant rendered as 32
//! hex chars — so all three consumers agree on object identity and a message
//! Himalaya adds deduplicates against the same message a sync stored.

use io_replica::object::ReplicaHash;

/// The content hash of a whole body (32 hex chars).
pub fn content_hash(bytes: &[u8]) -> ReplicaHash {
    let mut a: u64 = 0xcbf2_9ce4_8422_2325;
    let mut b: u64 = 0x9e37_79b9_7f4a_7c15;
    for &byte in bytes {
        a ^= byte as u64;
        a = a.wrapping_mul(0x0000_0100_0000_01b3);
        b = b.wrapping_add(byte as u64);
        b ^= b << 13;
        b = b.wrapping_mul(0xff51_afd7_ed55_8ccd);
    }
    a ^= bytes.len() as u64;
    ReplicaHash::from(format!("{a:016x}{b:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_shared_digest_shape() {
        // 32 lowercase hex chars, deterministic.
        let h = content_hash(b"Message-ID: <x@y>\r\n\r\nhello world");
        assert_eq!(h.0.len(), 32);
        assert!(h.0.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(content_hash(b"same"), content_hash(b"same"));
        assert_ne!(content_hash(b"a"), content_hash(b"b"));
    }
}
