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
