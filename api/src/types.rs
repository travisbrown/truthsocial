//! Request parameters and composite response types specific to the API surface.
//!
//! Individual entities (statuses, accounts, tags, …) reuse the [`truthsocial::model`] types; this
//! module only adds the wrappers the endpoints return (search results, status context) and the
//! pagination/filter parameters they accept.

use truthsocial::model::{Account, Group, Status, Tag};

/// Pagination and filtering parameters shared by the timeline-style endpoints.
///
/// Build one with [`TimelineParams::default`] and the chainable setters, e.g.
/// `TimelineParams::default().limit(20).max_id("123")`.
#[derive(Clone, Debug, Default)]
pub struct TimelineParams {
    max_id: Option<String>,
    since_id: Option<String>,
    min_id: Option<String>,
    limit: Option<u32>,
    exclude_replies: Option<bool>,
    only_media: Option<bool>,
    pinned: Option<bool>,
}

impl TimelineParams {
    /// Return only results older than this id (for paging backwards through a timeline).
    #[must_use]
    pub fn max_id(mut self, id: impl Into<String>) -> Self {
        self.max_id = Some(id.into());
        self
    }

    /// Return only results newer than this id.
    #[must_use]
    pub fn since_id(mut self, id: impl Into<String>) -> Self {
        self.since_id = Some(id.into());
        self
    }

    /// Return results immediately newer than this id.
    #[must_use]
    pub fn min_id(mut self, id: impl Into<String>) -> Self {
        self.min_id = Some(id.into());
        self
    }

    /// Maximum number of results to return (the API caps this, typically at 40).
    #[must_use]
    pub const fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Whether to omit replies (account-statuses endpoint).
    #[must_use]
    pub const fn exclude_replies(mut self, exclude: bool) -> Self {
        self.exclude_replies = Some(exclude);
        self
    }

    /// Whether to return only statuses with media attachments.
    #[must_use]
    pub const fn only_media(mut self, only: bool) -> Self {
        self.only_media = Some(only);
        self
    }

    /// Whether to return only the account's pinned statuses (account-statuses endpoint).
    #[must_use]
    pub const fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = Some(pinned);
        self
    }

    /// The set fields as query-string pairs (unset fields are omitted).
    pub(crate) fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(value) = &self.max_id {
            query.push(("max_id", value.clone()));
        }
        if let Some(value) = &self.since_id {
            query.push(("since_id", value.clone()));
        }
        if let Some(value) = &self.min_id {
            query.push(("min_id", value.clone()));
        }
        if let Some(value) = self.limit {
            query.push(("limit", value.to_string()));
        }
        if let Some(value) = self.exclude_replies {
            query.push(("exclude_replies", value.to_string()));
        }
        if let Some(value) = self.only_media {
            query.push(("only_media", value.to_string()));
        }
        if let Some(value) = self.pinned {
            query.push(("pinned", value.to_string()));
        }
        query
    }
}

/// The kind of entity to restrict a [`search`](crate::client::Client::search) to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchKind {
    /// Accounts (users and groups' owner accounts).
    Accounts,
    /// Statuses (posts).
    Statuses,
    /// Hashtags.
    Hashtags,
    /// Groups (Truth Social specific).
    Groups,
}

impl SearchKind {
    /// The `type` query-parameter value the API expects.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accounts => "accounts",
            Self::Statuses => "statuses",
            Self::Hashtags => "hashtags",
            Self::Groups => "groups",
        }
    }
}

/// The composite results of `GET /api/v2/search`.
///
/// Each list is empty when the search was restricted (via [`SearchKind`]) to a different kind, or
/// when there were simply no matches.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct SearchResults {
    /// Matching accounts.
    #[serde(default)]
    pub accounts: Vec<Account<'static>>,
    /// Matching statuses.
    #[serde(default)]
    pub statuses: Vec<Status<'static>>,
    /// Matching hashtags.
    #[serde(default)]
    pub hashtags: Vec<Tag<'static>>,
    /// Matching groups (Truth Social specific). Each carries a search-relevance
    /// [`position`](Group::position) not present when a group is embedded in a status.
    #[serde(default)]
    pub groups: Vec<Group<'static>>,
}

/// The ancestors and descendants of a status, from `GET /api/v1/statuses/:id/context`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    /// Statuses this one is a reply to, oldest first.
    pub ancestors: Vec<Status<'static>>,
    /// Replies (and their replies) to this status.
    pub descendants: Vec<Status<'static>>,
}
