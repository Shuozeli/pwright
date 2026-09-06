use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use pwright_core::{CallMetadata, Command, CommandResult, CommandSpec, ObjectId, Service};

use crate::remote::{FrameKind, PageKind, Remote};
use crate::{CallIdSource, CommandFuture, Frame, Page};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorScopeKind {
    Page,
    Frame,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatorScopeMismatch {
    left_id: ObjectId,
    left_kind: LocatorScopeKind,
    right_id: ObjectId,
    right_kind: LocatorScopeKind,
}

impl LocatorScopeMismatch {
    pub const fn left_id(self) -> ObjectId {
        self.left_id
    }

    pub const fn left_kind(self) -> LocatorScopeKind {
        self.left_kind
    }

    pub const fn right_id(self) -> ObjectId {
        self.right_id
    }

    pub const fn right_kind(self) -> LocatorScopeKind {
        self.right_kind
    }
}

impl Display for LocatorScopeMismatch {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot combine {:?} locator scope `{}` with {:?} scope `{}`",
            self.left_kind, self.left_id, self.right_kind, self.right_id
        )
    }
}

impl Error for LocatorScopeMismatch {}

enum LocatorScope<Channel, Ids> {
    Page(Remote<PageKind, Channel, Ids>),
    Frame(Remote<FrameKind, Channel, Ids>),
}

impl<Channel, Ids> LocatorScope<Channel, Ids> {
    const fn id(&self) -> ObjectId {
        match self {
            Self::Page(remote) => remote.id(),
            Self::Frame(remote) => remote.id(),
        }
    }

    const fn kind(&self) -> LocatorScopeKind {
        match self {
            Self::Page(_) => LocatorScopeKind::Page,
            Self::Frame(_) => LocatorScopeKind::Frame,
        }
    }

    fn is_connected(&self) -> bool {
        match self {
            Self::Page(remote) => remote.is_connected(),
            Self::Frame(remote) => remote.is_connected(),
        }
    }

    fn same_identity(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Page(left), Self::Page(right)) => left.has_same_identity(right),
            (Self::Frame(left), Self::Frame(right)) => left.has_same_identity(right),
            _ => false,
        }
    }
}

impl<Channel, Ids> Clone for LocatorScope<Channel, Ids> {
    fn clone(&self) -> Self {
        match self {
            Self::Page(remote) => Self::Page(remote.clone()),
            Self::Frame(remote) => Self::Frame(remote.clone()),
        }
    }
}

pub struct Locator<Channel, Ids, Query> {
    scope: LocatorScope<Channel, Ids>,
    query: Query,
}

impl<Channel, Ids, Query> Locator<Channel, Ids, Query> {
    pub const fn scope_id(&self) -> ObjectId {
        self.scope.id()
    }

    pub const fn scope_kind(&self) -> LocatorScopeKind {
        self.scope.kind()
    }

    pub fn is_connected(&self) -> bool {
        self.scope.is_connected()
    }

    pub const fn query(&self) -> &Query {
        &self.query
    }

    pub fn into_query(self) -> Query {
        self.query
    }

    pub fn map_query<Output>(
        self,
        map: impl FnOnce(Query) -> Output,
    ) -> Locator<Channel, Ids, Output> {
        Locator {
            scope: self.scope,
            query: map(self.query),
        }
    }

    pub fn combine<Other, Output>(
        self,
        other: Locator<Channel, Ids, Other>,
        combine: impl FnOnce(Query, Other) -> Output,
    ) -> Result<Locator<Channel, Ids, Output>, LocatorScopeMismatch> {
        if !self.scope.same_identity(&other.scope) {
            return Err(LocatorScopeMismatch {
                left_id: self.scope.id(),
                left_kind: self.scope.kind(),
                right_id: other.scope.id(),
                right_kind: other.scope.kind(),
            });
        }
        Ok(Locator {
            scope: self.scope,
            query: combine(self.query, other.query),
        })
    }

    /// Dispatches a typed locator command against this locator's page or frame.
    ///
    /// Selector/action extension crates own the concrete query and command
    /// types; the base client only preserves the locator's remote scope.
    pub fn dispatch<Spec>(
        &self,
        params: Spec::Params,
        metadata: CallMetadata,
    ) -> CommandFuture<Spec::Result, Spec::ErrorDetails>
    where
        Spec: CommandSpec,
        Spec::Params: Send + 'static,
        Ids: CallIdSource,
        Channel: Service<Command<Spec>, Response = CommandResult<Spec>, Error = std::convert::Infallible>
            + 'static,
    {
        match &self.scope {
            LocatorScope::Page(remote) => remote.dispatch::<Spec>(params, metadata),
            LocatorScope::Frame(remote) => remote.dispatch::<Spec>(params, metadata),
        }
    }
}

impl<Channel, Ids, Query> Clone for Locator<Channel, Ids, Query>
where
    Query: Clone,
{
    fn clone(&self) -> Self {
        Self {
            scope: self.scope.clone(),
            query: self.query.clone(),
        }
    }
}

impl<Channel, Ids, Query> Debug for Locator<Channel, Ids, Query>
where
    Query: Debug,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Locator")
            .field("scope_id", &self.scope.id())
            .field("scope_kind", &self.scope.kind())
            .field("connected", &self.scope.is_connected())
            .field("query", &self.query)
            .finish()
    }
}

impl<Channel, Ids, Query> PartialEq for Locator<Channel, Ids, Query>
where
    Query: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.scope.same_identity(&other.scope) && self.query == other.query
    }
}

impl<Channel, Ids, Query> Eq for Locator<Channel, Ids, Query> where Query: Eq {}

impl<Channel, Ids> Page<Channel, Ids> {
    pub fn locator<Query>(&self, query: Query) -> Locator<Channel, Ids, Query> {
        Locator {
            scope: self.main_frame().map_or_else(
                || LocatorScope::Page(self.remote.clone()),
                |frame| LocatorScope::Frame(frame.remote),
            ),
            query,
        }
    }
}

impl<Channel, Ids> Frame<Channel, Ids> {
    pub fn locator<Query>(&self, query: Query) -> Locator<Channel, Ids, Query> {
        Locator {
            scope: LocatorScope::Frame(self.remote.clone()),
            query,
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::ObjectId;

    use crate::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, FrameKind, PageKind,
        RemoteInitializer, RemoteKind,
    };

    use super::{LocatorScopeKind, LocatorScopeMismatch};

    #[test]
    fn locator_query_mapping_is_local_and_preserves_scope() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let page = client.page(ObjectId::new(7));
        let locator = page.locator("button".to_string());

        // Act
        let mapped = locator.map_query(|query| format!("css={query}"));

        // Assert
        assert_eq!(mapped.scope_id(), ObjectId::new(7));
        assert_eq!(mapped.scope_kind(), LocatorScopeKind::Page);
        assert_eq!(mapped.query(), "css=button");
    }

    #[test]
    fn locator_queries_combine_only_within_the_same_remote_scope() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let page = client.page(ObjectId::new(7));
        let left = page.locator("button");
        let right = page.locator("Submit");

        // Act
        let combined = left
            .combine(right, |left, right| (left, right))
            .expect("same-scope locators should combine");

        // Assert
        assert_eq!(combined.query(), &("button", "Submit"));
        assert_eq!(combined.scope_id(), ObjectId::new(7));
    }

    #[test]
    fn initialized_page_locators_normalize_to_their_main_frame_scope() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .expect("browser should initialize");
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .expect("context should initialize");
        let page = context
            .page_from_result(
                RemoteInitializer::new(
                    ObjectId::new(3),
                    Some(ObjectId::new(2)),
                    PageKind::OBJECT_KIND,
                ),
                RemoteInitializer::new(
                    ObjectId::new(4),
                    Some(ObjectId::new(3)),
                    FrameKind::OBJECT_KIND,
                ),
            )
            .expect("Page and main Frame should initialize");
        let main_frame = page.main_frame().expect("main Frame should be available");

        // Act
        let combined = page
            .locator("article")
            .combine(main_frame.locator("button"), |base, inner| (base, inner))
            .expect("Page and main Frame locators should share one scope");

        // Assert
        assert_eq!(combined.scope_kind(), LocatorScopeKind::Frame);
        assert_eq!(combined.scope_id(), ObjectId::new(4));
    }

    #[test]
    fn locator_composition_rejects_equal_ids_from_different_connections() {
        // Arrange
        let first_client = Client::new((), AtomicCallIdSource::new(1));
        let second_client = Client::new((), AtomicCallIdSource::new(1));
        let left = first_client.page(ObjectId::new(7)).locator("button");
        let right = second_client.page(ObjectId::new(7)).locator("Submit");

        // Act
        let error = left
            .combine(right, |left, right| (left, right))
            .expect_err("different connections must not combine locator queries");

        // Assert
        assert_eq!(
            error,
            LocatorScopeMismatch {
                left_id: ObjectId::new(7),
                left_kind: LocatorScopeKind::Page,
                right_id: ObjectId::new(7),
                right_kind: LocatorScopeKind::Page,
            }
        );
    }
}
