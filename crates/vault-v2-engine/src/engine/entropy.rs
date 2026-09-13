//! Operating-system CSPRNG access for Program 1A Task 2.
//!
//! Production code uses only the direct fallible `getrandom::fill` API. Partial
//! buffers are zeroized before a value-free error is returned.

use getrandom::fill;
use zeroize::Zeroize;

/// Value-free entropy failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntropyError;

/// The only production entropy source.
pub(crate) struct SystemEntropy;

impl SystemEntropy {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn fill32(&self, out: &mut [u8; 32]) -> Result<(), EntropyError> {
        fill_slice(out)
    }

    pub(crate) fn fill24(&self, out: &mut [u8; 24]) -> Result<(), EntropyError> {
        fill_slice(out)
    }

    pub(crate) fn fill16(&self, out: &mut [u8; 16]) -> Result<(), EntropyError> {
        fill_slice(out)
    }
}

fn fill_slice<T: Zeroize + AsMut<[u8]>>(out: &mut T) -> Result<(), EntropyError> {
    if fill(out.as_mut()).is_err() {
        out.zeroize();
        return Err(EntropyError);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) struct InjectingEntropy {
    remaining: std::cell::Cell<usize>,
}

#[cfg(test)]
impl InjectingEntropy {
    pub(crate) fn fail_after(successful_fills: usize) -> Self {
        Self {
            remaining: std::cell::Cell::new(successful_fills),
        }
    }

    pub(crate) fn fill32(&self, out: &mut [u8; 32]) -> Result<(), EntropyError> {
        self.fill(out)
    }

    pub(crate) fn fill24(&self, out: &mut [u8; 24]) -> Result<(), EntropyError> {
        self.fill(out)
    }

    pub(crate) fn fill16(&self, out: &mut [u8; 16]) -> Result<(), EntropyError> {
        self.fill(out)
    }

    fn fill<T: Zeroize + AsMut<[u8]>>(&self, out: &mut T) -> Result<(), EntropyError> {
        let left = self.remaining.get();
        if left == 0 {
            out.zeroize();
            return Err(EntropyError);
        }
        self.remaining.set(left - 1);
        fill_slice(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_entropy_fills_supported_buffer_sizes() {
        let entropy = SystemEntropy::new();
        let mut b32 = [0u8; 32];
        let mut b24 = [0u8; 24];
        let mut b16 = [0u8; 16];
        entropy.fill32(&mut b32).expect("fill32");
        entropy.fill24(&mut b24).expect("fill24");
        entropy.fill16(&mut b16).expect("fill16");
        assert_ne!(b32, [0u8; 32]);
        assert_ne!(b24, [0u8; 24]);
        assert_ne!(b16, [0u8; 16]);
    }

    #[test]
    fn injecting_entropy_zeroizes_and_fails_without_leaving_partial_secret() {
        let entropy = InjectingEntropy::fail_after(0);
        let mut out = [0xab; 32];
        assert!(entropy.fill32(&mut out).is_err());
        assert_eq!(out, [0u8; 32]);

        let entropy = InjectingEntropy::fail_after(1);
        let mut first = [0u8; 16];
        entropy.fill16(&mut first).expect("first fill ok");
        let mut second = [0xcd; 24];
        assert!(entropy.fill24(&mut second).is_err());
        assert_eq!(second, [0u8; 24]);
    }
}
