//! Boot sequence: lock, then live signer, then the sole redb open.
//!
//! Sessions and the one-credential registry stay in memory. Restart drops
//! them and rotates the signer epoch.

use std::path::Path;

use sovereign_authority::broker::store::OwnedStore;

use crate::approval_bridge::ApprovalBridge;
use crate::owner_surface::OwnerSurface;
use crate::trust_persist::{HistoricalTrust, TrustError};
use crate::{BoundaryError, ProcessBoundary};

/// Lock held, live signer generated, redb not yet opened.
pub struct LockedSigner {
    boundary: ProcessBoundary,
    bridge: ApprovalBridge,
}

impl std::fmt::Debug for LockedSigner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LockedSigner")
            .field("root", &"<redacted>")
            .field("bridge", &self.bridge)
            .finish_non_exhaustive()
    }
}

impl LockedSigner {
    pub(crate) fn from_boundary(boundary: ProcessBoundary) -> Result<Self, BoundaryError> {
        let bridge = ApprovalBridge::generate()?;
        Ok(Self { boundary, bridge })
    }

    pub fn boundary(&self) -> &ProcessBoundary {
        &self.boundary
    }

    pub fn bridge(&self) -> &ApprovalBridge {
        &self.bridge
    }

    /// The only production redb open on this path, after the signer exists.
    pub fn persist(self) -> Result<FixtureOwner, BoundaryError> {
        let record = {
            let store = self.boundary.open_store()?;
            self.bridge
                .persist_into(&store)
                .map_err(|_| BoundaryError::StoreUnavailable)?
        };
        let _ = record;
        let epoch = self.bridge.signer_epoch();
        let generation = self.boundary.generation();
        let mut surface = OwnerSurface::new();
        surface.bind_live_signer(epoch, generation);
        Ok(FixtureOwner {
            boundary: self.boundary,
            bridge: self.bridge,
            surface,
        })
    }
}

/// In-process unqualified owner fixture: sessions, registry, live bridge.
pub struct FixtureOwner {
    boundary: ProcessBoundary,
    bridge: ApprovalBridge,
    surface: OwnerSurface,
}

impl std::fmt::Debug for FixtureOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FixtureOwner")
            .field("root", &"<redacted>")
            .field("bridge", &self.bridge)
            .finish_non_exhaustive()
    }
}

impl FixtureOwner {
    pub fn boot(root: &Path) -> Result<Self, BoundaryError> {
        ProcessBoundary::acquire(root)?.with_signer()?.persist()
    }

    pub fn boundary(&self) -> &ProcessBoundary {
        &self.boundary
    }

    pub fn bridge(&self) -> &ApprovalBridge {
        &self.bridge
    }

    pub fn surface(&self) -> &OwnerSurface {
        &self.surface
    }

    pub fn surface_mut(&mut self) -> &mut OwnerSurface {
        &mut self.surface
    }

    pub fn open_store(&self) -> Result<OwnedStore<'_>, BoundaryError> {
        self.boundary.open_store()
    }

    pub fn load_historical_trust(&self) -> Result<HistoricalTrust, TrustError> {
        let store = self
            .boundary
            .open_store()
            .map_err(|_| TrustError::Unavailable)?;
        HistoricalTrust::load(&store)
    }
}
