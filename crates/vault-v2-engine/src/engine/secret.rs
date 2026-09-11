//! Key holders that cannot be copied, printed, or left behind.
//!
//! A `DbKey` is 32 random bytes and opens the business store. Everything
//! about this type is shaped by one question — where can those bytes end up
//! other than where they are needed — and the answer it enforces is: nowhere.
//! It has no `Clone`, so there is exactly one copy and it is zeroed on drop;
//! no `Debug` or `Display`, so it cannot reach a log line or a panic message;
//! and no serde, so it cannot be written anywhere by accident. The tests at
//! the bottom prove the absences at compile time rather than by inspection.

use zeroize::{Zeroize, ZeroizeOnDrop};

/// The random 32-byte database key.
#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct DbKey([u8; 32]);

impl DbKey {
    /// Take ownership of key bytes. The caller's array is copied in; callers
    /// holding the source should zero it themselves, and the constructor's
    /// argument is consumed so no binding to it survives here.
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The key in SQLCipher's raw-key form, for the one FFI call that uses it.
    pub(crate) fn raw(&self) -> RawSqlcipherKey {
        RawSqlcipherKey::encode(&self.0)
    }
}

/// Length of SQLCipher's raw-key token: `x'`, 64 hex digits, `'`.
pub(crate) const RAW_KEY_LEN: usize = 2 + 64 + 1;

/// A DBK encoded as SQLCipher's raw-key token.
///
/// SQLCipher treats a key of exactly the form `x'<64 hex digits>'` as a raw
/// 256-bit key and skips its password KDF. That is the right mode here: the
/// DBK is already 32 random bytes, so stretching it would add cost and no
/// strength. The token is still key material — it contains every bit of the
/// DBK — so it gets the same treatment as the key itself.
#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct RawSqlcipherKey([u8; RAW_KEY_LEN]);

impl RawSqlcipherKey {
    fn encode(key: &[u8; 32]) -> Self {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut token = [0u8; RAW_KEY_LEN];
        token[0] = b'x';
        token[1] = b'\'';
        for (index, byte) in key.iter().enumerate() {
            token[2 + index * 2] = HEX[usize::from(byte >> 4)];
            token[3 + index * 2] = HEX[usize::from(byte & 0x0f)];
        }
        token[RAW_KEY_LEN - 1] = b'\'';
        Self(token)
    }

    /// The token bytes, for passing to `sqlite3_key_v2` with an explicit
    /// length. There is no terminating NUL: the length is passed, and a NUL
    /// inside a key is exactly the byte a C string API would truncate at.
    pub(crate) fn token_bytes(&self) -> &[u8; RAW_KEY_LEN] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::{assert_impl_all, assert_not_impl_any};

    // The absences, proven by the compiler. A derive added to either type
    // later fails the build here rather than leaking a key into a log.
    assert_not_impl_any!(DbKey: Clone, Copy, std::fmt::Debug, std::fmt::Display);
    assert_not_impl_any!(RawSqlcipherKey: Clone, Copy, std::fmt::Debug, std::fmt::Display);

    // And the intended presence: both are zeroed when they go out of scope.
    assert_impl_all!(DbKey: ZeroizeOnDrop);
    assert_impl_all!(RawSqlcipherKey: ZeroizeOnDrop);

    /// The edge bytes the plan names: the smallest, one that needs a leading
    /// zero digit, a mixed one, and the largest. Each is checked byte by byte
    /// rather than by comparing against another encoder, because a second
    /// encoder would share whatever mistake the first one made.
    #[test]
    fn raw_key_encoding_is_exactly_67_bytes_for_edge_bytes() {
        for (fill, digits) in [(0x00u8, b"00"), (0x0f, b"0f"), (0xab, b"ab"), (0xff, b"ff")] {
            let raw = DbKey::from_bytes([fill; 32]).raw();
            let token = raw.token_bytes();

            assert_eq!(token.len(), 67);
            assert_eq!(&token[..2], b"x'", "prefix for 0x{fill:02x}");
            assert_eq!(token[66], b'\'', "suffix for 0x{fill:02x}");
            for pair in token[2..66].chunks(2) {
                assert_eq!(pair, digits, "hex pair for 0x{fill:02x}");
            }
            // A NUL would be silently truncated by any C string path the
            // token might one day pass through — including 0x00 keys, whose
            // hex digits are the character '0', not the byte zero.
            assert!(
                !token.contains(&0),
                "a NUL byte in the token for 0x{fill:02x}"
            );
        }
    }

    /// Distinct keys give distinct tokens, and the position of each byte is
    /// preserved — a transposition bug would pass the uniform-fill vectors
    /// above and fail here.
    #[test]
    fn raw_key_encoding_preserves_byte_order() {
        let mut bytes = [0u8; 32];
        for (index, slot) in bytes.iter_mut().enumerate() {
            *slot = u8::try_from(index).expect("index fits a byte");
        }
        let raw = DbKey::from_bytes(bytes).raw();
        let hex = std::str::from_utf8(&raw.token_bytes()[2..66]).expect("ascii hex");
        assert_eq!(
            hex,
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        );
    }
}
