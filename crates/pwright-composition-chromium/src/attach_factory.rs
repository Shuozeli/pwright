use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_backend_cdp::ConnectionEpoch;
use pwright_core::{AtomicObjectIdSource, ObjectId, ObjectIdSourceError, ResourceLimits};
use pwright_provider_cdp::CdpWebSocketEndpoint;

use crate::{
    AttachedChromiumComposition, ChromiumAttachError, ChromiumAttachProductionProfile,
    ChromiumCompositionError, PreparedChromiumAttach,
};

/// Reusable declarative recipe for fresh Chromium attachments.
///
/// The factory retains only immutable configuration. Every preparation creates
/// a distinct connection-local resource ledger and connector; every attachment
/// additionally creates a distinct CDP connection, topology, object-ID source,
/// and initial connection epoch. Reusing this factory therefore never rebinds
/// an existing Browser graph to another native connection.
#[derive(Clone, Debug)]
pub struct ChromiumAttachFactory {
    profile: ChromiumAttachProductionProfile,
    limits: ResourceLimits,
    browser_object_id: ObjectId,
}

impl ChromiumAttachFactory {
    /// Validates the reusable recipe without opening a browser connection.
    pub fn new(
        profile: ChromiumAttachProductionProfile,
        limits: ResourceLimits,
        browser_object_id: ObjectId,
    ) -> Result<Self, ChromiumAttachFactoryError> {
        validate_browser_object_namespace(browser_object_id)?;
        let factory = Self {
            profile,
            limits,
            browser_object_id,
        };
        factory.prepare()?;
        Ok(factory)
    }

    pub const fn profile(&self) -> &ChromiumAttachProductionProfile {
        &self.profile
    }

    pub const fn limits(&self) -> ResourceLimits {
        self.limits
    }

    /// Returns the Browser root ID reused only across independent wire object
    /// namespaces. Objects from separate factory attachments are unrelated
    /// even when their numeric IDs match.
    pub const fn browser_object_id(&self) -> ObjectId {
        self.browser_object_id
    }

    /// Each attachment starts a new topology history. Later epochs are
    /// reserved for reconstruction within that same live CDP connection.
    pub const fn initial_epoch(&self) -> ConnectionEpoch {
        ConnectionEpoch::new(1)
    }

    /// Creates a fresh, connection-local preparation without opening a socket.
    pub fn prepare(&self) -> Result<PreparedChromiumAttach, ChromiumAttachFactoryError> {
        PreparedChromiumAttach::new(self.profile.clone(), self.limits)
            .map_err(ChromiumAttachFactoryError::Preparation)
    }

    /// Opens a new native attachment with a fresh resource ledger, topology,
    /// object graph namespace, and CDP event view.
    pub async fn attach(
        &self,
        endpoint: CdpWebSocketEndpoint,
    ) -> Result<AttachedChromiumComposition, ChromiumAttachFactoryError> {
        self.prepare()?
            .attach(endpoint, self.browser_object_id, self.initial_epoch())
            .await
            .map_err(ChromiumAttachFactoryError::Attachment)
    }
}

fn validate_browser_object_namespace(
    browser_object_id: ObjectId,
) -> Result<(), ChromiumAttachFactoryError> {
    let first_child = browser_object_id
        .get()
        .checked_add(1)
        .ok_or(ObjectIdSourceError::Exhausted)?;
    AtomicObjectIdSource::new(first_child)?;
    Ok(())
}

#[derive(Debug)]
pub enum ChromiumAttachFactoryError {
    ObjectIds(ObjectIdSourceError),
    Preparation(ChromiumCompositionError),
    Attachment(ChromiumAttachError),
}

impl Display for ChromiumAttachFactoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectIds(error) => Display::fmt(error, formatter),
            Self::Preparation(error) => Display::fmt(error, formatter),
            Self::Attachment(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumAttachFactoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ObjectIds(error) => Some(error),
            Self::Preparation(error) => Some(error),
            Self::Attachment(error) => Some(error),
        }
    }
}

impl From<ObjectIdSourceError> for ChromiumAttachFactoryError {
    fn from(error: ObjectIdSourceError) -> Self {
        Self::ObjectIds(error)
    }
}
