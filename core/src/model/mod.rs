//! Core types for the Truth Social API.
//!
//! This module provides Rust bindings for deserializing (and serializing) Truth Social status
//! (post) data and related types: accounts, media attachments, mentions, tags, preview cards,
//! groups, polls, custom emoji, and Truth Social specific extensions such as TV metadata and
//! advertising metrics.
//!
//! String fields use [`Cow`] so the same types can model both freshly parsed API responses and
//! borrowed Wayback Machine archive snapshots. The archive integration (snapshot configuration and
//! parsing) lives in the separate `truthsocial-wbm` crate, which depends on this one.

use chrono::{DateTime, NaiveDate, Utc};
use either::Either;
use serde_field_attributes::{integer_str, optional_integer_str, optional_usize};
use std::borrow::Cow;

/// A Truth Social status (post).
///
/// This is the primary content type, representing a single post on the platform. Statuses can
/// contain text content, media attachments, mentions, and hashtags. They may also be replies to
/// other statuses, quotes, or reblogs.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Status<'a> {
    /// Unique identifier for this status.
    #[serde(with = "integer_str")]
    pub id: u64,

    /// Timestamp when this status was created.
    pub created_at: DateTime<Utc>,

    /// ID of the status this is replying to, if any.
    #[serde(default, with = "optional_integer_str")]
    pub in_reply_to_id: Option<u64>,

    /// ID of a quoted status, if any.
    #[serde(default, with = "optional_integer_str")]
    pub quote_id: Option<u64>,

    /// ID of the account being replied to, if any.
    #[serde(default, with = "optional_integer_str")]
    pub in_reply_to_account_id: Option<u64>,

    /// Whether this status contains sensitive content.
    pub sensitive: bool,

    /// Content warning text, if any.
    pub spoiler_text: Cow<'a, str>,

    /// Visibility level of this status, or `None` when the capture omits it. Older records
    /// (and some Wayback Machine snapshots) serialize `visibility` as `null`.
    pub visibility: Option<Visibility>,

    /// ISO 639-1 language code, if detected.
    pub language: Option<Cow<'a, str>>,

    /// `ActivityPub` URI for this status.
    pub uri: Cow<'a, str>,

    /// Web URL for this status.
    pub url: Cow<'a, str>,

    /// HTML-formatted content of this status.
    pub content: Cow<'a, str>,

    /// Plain-text source of the content. Only ever observed as `null`, so its type is unknown;
    /// deserialization fails loudly if a value ever appears, prompting the real type.
    pub text: Option<()>,

    /// The application used to post this status. Only ever observed as `null`, so its type is
    /// unknown; deserialization fails loudly if a value ever appears, prompting the real type.
    pub application: Option<()>,

    /// The account that authored this status.
    pub account: Account<'a>,

    /// Media attachments (images, videos, etc.).
    pub media_attachments: Vec<MediaAttachment<'a>>,

    /// Accounts mentioned in this status.
    pub mentions: Vec<Mention<'a>>,

    /// Hashtags used in this status.
    pub tags: Vec<Tag<'a>>,

    /// Preview card for linked content, if any.
    pub card: Option<Card<'a>>,

    /// Group this status was posted to, if any.
    pub group: Option<Group<'a>>,

    /// Quoted status, if this is a quote post.
    pub quote: Option<Box<Self>>,

    /// The status being replied to, if this is a reply. Note: This is a Truth Social extension, not
    /// part of standard Mastodon API.
    pub in_reply_to: Option<Box<Self>>,

    /// Reblogged status, if this is a reblog/boost.
    pub reblog: Option<Box<Self>>,

    /// Whether this is a sponsored/promoted status.
    pub sponsored: Option<bool>,

    // These three counts use `optional_usize`: the API returns `-1` as a sentinel when a count is
    // withheld (e.g. for some nested/reblogged statuses) rather than omitting the field, and that
    // sentinel deserializes to `None`.
    /// Number of replies to this status (`None` if the API withheld the count).
    #[serde(with = "optional_usize")]
    pub replies_count: Option<usize>,

    /// Number of times this status has been reblogged (`None` if the API withheld the count).
    #[serde(with = "optional_usize")]
    pub reblogs_count: Option<usize>,

    /// Number of times this status has been favourited (`None` if the API withheld the count).
    #[serde(with = "optional_usize")]
    pub favourites_count: Option<usize>,

    /// Current user's reaction to this status, if any (e.g. `"upvote"`).
    pub reaction: Option<Cow<'a, str>>,

    /// Number of upvotes (Truth Social specific).
    pub upvotes_count: Option<u64>,

    /// Number of downvotes (Truth Social specific).
    pub downvotes_count: Option<u64>,

    /// Whether the current user has favourited this status.
    pub favourited: Option<bool>,

    /// Whether the current user has reblogged this status.
    pub reblogged: Option<bool>,

    /// Whether the current user has muted this status.
    pub muted: Option<bool>,

    /// Whether the current user has muted this status's quoted status (Truth Social specific).
    pub quote_muted: Option<bool>,

    /// Whether this status is visible in its group's timeline (Truth Social specific).
    pub group_timeline_visible: Option<bool>,

    /// Whether this status is pinned to the author's profile.
    pub pinned: Option<bool>,

    /// Whether the current user has bookmarked this status.
    pub bookmarked: Option<bool>,

    /// Poll attached to this status, if any.
    pub poll: Option<Poll<'a>>,

    /// Custom emoji used in this status.
    pub emojis: Vec<Emoji<'a>>,

    /// Whether the current user can vote on this status (Truth Social specific).
    pub votable: Option<bool>,

    /// Timestamp when this status was last edited, if ever.
    pub edited_at: Option<DateTime<Utc>>,

    /// Version number of this status (for edit tracking).
    pub version: Option<Cow<'a, str>>,

    /// Whether the current user can edit this status.
    pub editable: Option<bool>,

    /// Title of this status (Truth Social specific).
    pub title: Option<Cow<'a, str>>,

    /// Tombstone information if this status was deleted.
    pub tombstone: Option<Tombstone>,

    /// Timestamp when this status was deleted, if applicable.
    pub deleted_at: Option<DateTime<Utc>>,

    /// Advertising metrics for sponsored content (Truth Social specific).
    pub metrics: Option<Metrics<'a>>,

    /// TV-specific metadata for live/scheduled TV content (Truth Social specific).
    pub tv: Option<TvMetadata<'a>>,

    /// Announcement embedded in this status (Truth Social specific). Absent in older snapshots; its
    /// non-null shape has not been observed, so it is kept as an opaque value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedded_announcement: Option<serde_json::Value>,

    /// URL of the next status in a paged video feed (Truth Social specific). Absent in older
    /// snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_status: Option<Cow<'a, str>>,

    /// Per-request signature token. Observed as `<base64-json>.<base64-other>`, where the JSON
    /// payload decodes to `[viewer_id, status_id, request_timestamp]`. Absent in older snapshots;
    /// kept as an opaque token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Cow<'a, str>>,

    /// Whether this reply is pinned beneath its parent status by that status's author. Absent in
    /// older snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_pinned: Option<bool>,
}

/// The `content` of a snapshot: a single [`Status`], or an array of them (some endpoints, such as
/// paged video feeds, return a list).
#[derive(Clone, Debug, PartialEq)]
pub enum StatusContent<'a> {
    /// A single status object.
    Single(Box<Status<'a>>),
    /// An array of status objects.
    Multiple(Vec<Status<'a>>),
}

impl<'a> StatusContent<'a> {
    /// Borrows each status in the content: one for [`Single`](Self::Single), all for
    /// [`Multiple`](Self::Multiple).
    pub fn statuses(&self) -> impl Iterator<Item = &Status<'a>> {
        match self {
            // `Either` keeps the two arms a single iterator type without allocating for `Single`.
            Self::Single(status) => Either::Left(std::iter::once(status.as_ref())),
            Self::Multiple(statuses) => Either::Right(statuses.iter()),
        }
    }

    /// Consumes the content, yielding each owned status: one for [`Single`](Self::Single), all for
    /// [`Multiple`](Self::Multiple).
    pub fn into_statuses(self) -> impl Iterator<Item = Status<'a>> {
        match self {
            Self::Single(status) => Either::Left(std::iter::once(*status)),
            Self::Multiple(statuses) => Either::Right(statuses.into_iter()),
        }
    }
}

// A hand-written `Deserialize` (rather than `#[serde(untagged)]`) that dispatches on the JSON shape
// via `deserialize_any`: an object is a single status, an array is many. The untagged derive
// buffers into an intermediate value, through which `Status`'s field-level custom deserializers
// (e.g. the `integer_str` and `last_status_at` helpers) do not round-trip. `'a` is named (not
// elided) because the inner visitor refers to it.
#[allow(clippy::elidable_lifetime_names)]
impl<'de, 'a> serde::Deserialize<'de> for StatusContent<'a> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ContentVisitor<'a>(std::marker::PhantomData<&'a ()>);

        impl<'de, 'a> serde::de::Visitor<'de> for ContentVisitor<'a> {
            type Value = StatusContent<'a>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a status object or an array of statuses")
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                <Status<'a> as serde::Deserialize>::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )
                .map(|status| StatusContent::Single(Box::new(status)))
            }

            fn visit_seq<S: serde::de::SeqAccess<'de>>(
                self,
                seq: S,
            ) -> Result<Self::Value, S::Error> {
                <Vec<Status<'a>> as serde::Deserialize>::deserialize(
                    serde::de::value::SeqAccessDeserializer::new(seq),
                )
                .map(StatusContent::Multiple)
            }
        }

        deserializer.deserialize_any(ContentVisitor(std::marker::PhantomData))
    }
}

/// Tombstone information for a deleted status.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tombstone {
    /// Reason for deletion.
    pub reason: TombstoneReason,
}

/// Advertising metrics for sponsored content.
///
/// Present on sponsored statuses to track ad impressions and display information.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics<'a> {
    /// URL to call for tracking impressions.
    pub impression: Option<Cow<'a, str>>,

    /// When these metrics expire.
    pub expires_at: Option<DateTime<Utc>>,

    /// Reason displayed to user for why this ad is shown.
    pub reason: Option<Cow<'a, str>>,
}

/// TV-specific metadata for live or scheduled TV content.
///
/// Present on statuses and media attachments related to Truth Social TV.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TvMetadata<'a> {
    /// Channel identifier.
    pub channel_id: u64,

    /// When the TV program starts.
    pub start_time: DateTime<Utc>,

    /// When the TV program ends.
    pub end_time: DateTime<Utc>,

    /// Name/title of the TV program.
    pub name: Cow<'a, str>,

    /// PLTV timespan in milliseconds (can be negative for past programs).
    pub pltv_timespan: i64,

    /// URL to the program's image/thumbnail.
    pub image_url: Cow<'a, str>,

    /// Description of the TV program.
    pub description: Cow<'a, str>,
}

/// Reason why a status was tombstoned.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TombstoneReason {
    /// The status was deleted by the user or a moderator.
    Deleted,
}

/// Visibility level of a status.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Visible to everyone.
    Public,
    /// Visible to followers only.
    Private,
    /// Visible only to mentioned users.
    Direct,
    /// Public but not shown in public timelines.
    Unlisted,
    /// Visible only to group members.
    Group,
    /// Visible only to the author (self).
    #[serde(rename = "self")]
    Self_,
}

/// A Truth Social user account.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Account<'a> {
    /// Unique identifier for this account.
    #[serde(with = "integer_str")]
    pub id: u64,

    /// Username (without domain).
    pub username: Cow<'a, str>,

    /// Webfinger account URI (username@domain for remote, username for local).
    pub acct: Cow<'a, str>,

    /// Display name shown on profile.
    pub display_name: Cow<'a, str>,

    /// Whether this account requires follow approval.
    pub locked: bool,

    /// Whether this account is a bot.
    pub bot: bool,

    /// Whether this account has opted into discovery features.
    pub discoverable: Option<bool>,

    /// Whether this is a group account.
    pub group: bool,

    /// Timestamp when this account was created.
    pub created_at: DateTime<Utc>,

    /// HTML-formatted bio/description.
    pub note: Cow<'a, str>,

    /// URL to the account's profile page.
    pub url: Cow<'a, str>,

    /// URL to the account's avatar image.
    pub avatar: Cow<'a, str>,

    /// URL to the static version of the avatar.
    pub avatar_static: Cow<'a, str>,

    /// URL to the account's header/banner image.
    pub header: Cow<'a, str>,

    /// URL to the static version of the header.
    pub header_static: Cow<'a, str>,

    /// Number of followers.
    pub followers_count: u64,

    /// Number of accounts this account follows.
    pub following_count: u64,

    /// Number of statuses posted (occasionally negative for unknown reasons).
    pub statuses_count: i64,

    /// Timestamp of the most recent status, if any.
    ///
    /// Truth Social returns this inconsistently as either a date (`"2023-05-23"`, [`Either::Left`])
    /// or a full RFC 3339 datetime (`"2023-05-23T22:31:45.360Z"`, [`Either::Right`]); both forms
    /// are preserved.
    #[serde(default, with = "either::serde_untagged_optional")]
    pub last_status_at: Option<Either<NaiveDate, DateTime<Utc>>>,

    /// Whether this account is verified (Truth Social specific).
    pub verified: bool,

    /// Location string (Truth Social specific).
    pub location: Cow<'a, str>,

    /// Website URL (Truth Social specific).
    pub website: Cow<'a, str>,

    /// Whether unauthenticated users can view this account (Truth Social specific).
    pub unauth_visibility: Option<bool>,

    /// Whether this account has completed chat onboarding (Truth Social specific).
    pub chats_onboarded: Option<bool>,

    /// Whether this account has completed feeds onboarding (Truth Social specific).
    pub feeds_onboarded: Option<bool>,

    /// Whether this account has completed bookmarks onboarding (Truth Social specific).
    pub bookmarks_onboarded: Option<bool>,

    /// Whether this account has completed group-reactions onboarding (Truth Social specific).
    pub group_reactions_onboarded: Option<bool>,

    /// Whether this account accepts direct messages (Truth Social specific).
    pub accepting_messages: Option<bool>,

    /// Whether this account only receives mentions from accounts it follows (Truth Social
    /// specific).
    pub receive_only_follow_mentions: Option<bool>,

    /// Whether this account has accepted the status-edit prompt (Truth Social specific).
    pub accepted_status_edit_prompt: Option<bool>,

    /// Whether to show non-member group statuses (Truth Social specific).
    pub show_nonmember_group_statuses: Option<bool>,

    /// Custom emoji used by this account.
    pub emojis: Vec<Emoji<'a>>,

    /// Profile metadata fields.
    pub fields: Vec<Field<'a>>,

    /// Whether this account has completed TV onboarding (Truth Social specific).
    pub tv_onboarded: Option<bool>,

    /// Whether this is a TV account (Truth Social specific).
    pub tv_account: Option<bool>,

    /// Whether this account has premium subscription (Truth Social specific).
    pub premium: Option<bool>,

    /// Whether this account is suspended.
    pub suspended: Option<bool>,

    /// Pleroma-specific account data (legacy compatibility field).
    pub pleroma: Option<Pleroma>,

    /// The account this one has moved to. Only ever observed as `null`, so its type is unknown;
    /// deserialization fails loudly if a value ever appears, prompting the real type.
    pub moved: Option<()>,
}

/// Pleroma-specific account data.
///
/// This is a legacy compatibility field from Mastodon/Pleroma federation.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pleroma {
    /// Whether this account accepts chat messages.
    pub accepts_chat_messages: Option<bool>,
}

/// A media attachment (image, video, audio, etc.).
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MediaAttachment<'a> {
    /// Unique identifier for this attachment.
    #[serde(with = "integer_str")]
    pub id: u64,

    /// Type of media.
    #[serde(rename = "type")]
    pub media_type: MediaType,

    /// URL to the full-size media.
    pub url: Cow<'a, str>,

    /// URL to the preview/thumbnail.
    pub preview_url: Cow<'a, str>,

    /// External video ID for embedded videos, if any.
    pub external_video_id: Option<Cow<'a, str>>,

    /// URL to the remote original, if federated.
    pub remote_url: Option<Cow<'a, str>>,

    /// URL to the remote preview, if federated.
    pub preview_remote_url: Option<Cow<'a, str>>,

    /// URL for text representation, if any.
    pub text_url: Option<Cow<'a, str>>,

    /// Metadata about the media dimensions and processing.
    pub meta: MediaMeta<'a>,

    /// Alt text description.
    pub description: Option<Cow<'a, str>>,

    /// Blurhash placeholder string, if available.
    pub blurhash: Option<Cow<'a, str>>,

    /// Processing status of this attachment.
    pub processing: Option<ProcessingStatus>,

    /// TV-specific metadata for live/scheduled TV content.
    pub tv: Option<TvMetadata<'a>>,
}

/// Type of media attachment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    /// Static image.
    Image,
    /// Animated GIF.
    Gifv,
    /// Video file.
    Video,
    /// Audio file.
    Audio,
    /// Truth Social TV stream.
    Tv,
}

/// Processing status of a media attachment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    /// Queued for processing but not yet started.
    Queued,
    /// Processing is in progress.
    Processing,
    /// Processing is complete.
    Complete,
    /// Processing failed.
    Failed,
}

/// Metadata about media dimensions.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MediaMeta<'a> {
    /// Original size metadata.
    pub original: Option<ImageMeta<'a>>,

    /// Small/thumbnail size metadata.
    pub small: Option<ImageMeta<'a>>,

    /// Color information extracted from the media.
    pub colors: Option<MediaColors<'a>>,
}

/// Color information extracted from media.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct MediaColors<'a> {
    /// Accent color as hex string (e.g., "#c2aca1").
    pub accent: Cow<'a, str>,

    /// Background color as hex string.
    pub background: Cow<'a, str>,

    /// Foreground color as hex string.
    pub foreground: Cow<'a, str>,
}

/// Dimension metadata for an image or video.
///
/// For images, contains aspect ratio, dimensions, and size string. For videos, contains bitrate,
/// duration, frame rate, and dimensions.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageMeta<'a> {
    /// Aspect ratio (width / height). Present for images.
    pub aspect: Option<f64>,

    /// Height in pixels.
    pub height: Option<u64>,

    /// Size as a `WxH` string. Present for images.
    pub size: Option<Cow<'a, str>>,

    /// Width in pixels.
    pub width: Option<u64>,

    /// Bitrate in bits per second. Present for videos.
    pub bitrate: Option<u64>,

    /// Duration in seconds. Present for videos.
    pub duration: Option<f64>,

    /// Frame rate as a fraction string (e.g., "30000/1001"). Present for videos.
    pub frame_rate: Option<Cow<'a, str>>,
}

/// ID type for mentions, which can be a normal account ID or a sentinel value.
///
/// The Truth Social API uses `-99` as a sentinel value for domain-level mentions (e.g., mentioning
/// `@truthsocial.com` rather than a specific user account).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MentionId {
    /// A normal account ID.
    Account(u64),
    /// A domain-level mention (serialized as "-99").
    Domain,
}

impl<'de> serde::Deserialize<'de> for MentionId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // The ID comes as a string (e.g., "12345" or "-99"). Deserialize through `Cow` rather than
        // `&str`: a borrowed `&str` only works with input the deserializer can borrow from, so a
        // plain `&str` here would fail under owned or streaming deserializers such as
        // `serde_json::from_reader` or `from_value`. `Cow` accepts both borrowed and owned input.
        let s = Cow::<'de, str>::deserialize(deserializer)?;

        if s == "-99" {
            Ok(Self::Domain)
        } else {
            s.parse::<u64>()
                .map(Self::Account)
                .map_err(serde::de::Error::custom)
        }
    }
}

impl serde::Serialize for MentionId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Account(id) => serializer.serialize_str(&id.to_string()),
            Self::Domain => serializer.serialize_str("-99"),
        }
    }
}

/// A mention of another account in a status.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mention<'a> {
    /// Account ID of the mentioned user, or a sentinel value for domain mentions.
    pub id: MentionId,

    /// Username of the mentioned user.
    pub username: Cow<'a, str>,

    /// URL to the mentioned user's profile.
    pub url: Cow<'a, str>,

    /// Webfinger account URI.
    pub acct: Cow<'a, str>,
}

/// A hashtag used in a status.
///
/// Unlike the rest of the model, this type is not strict. `deny_unknown_fields` is incompatible with
/// the flattened [`trending`](Self::trending) field, and `#[serde(flatten)]` routes leftover keys
/// through Serde's content buffer, which ignores `deny_unknown_fields` even on the flattened struct.
/// Unknown keys are therefore silently dropped here rather than rejected.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Tag<'a> {
    /// The hashtag text (without the # symbol).
    pub name: Cow<'a, str>,

    /// URL to view statuses with this hashtag.
    pub url: Option<Cow<'a, str>>,

    /// Per-day usage history buckets, present in the tag-search and trending views (Mastodon
    /// standard) and absent from the lightweight tags inlined in a status. Skipped when absent so
    /// an inline tag still serializes to just `{name, url}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<TagHistory<'a>>>,

    /// Recent daily usage counts, newest first (Truth Social specific). Present in the tag-search
    /// view alongside [`history`](Self::history).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_history: Option<Vec<u64>>,

    /// Number of statuses that recently used this hashtag (Truth Social specific). Present in the
    /// tag-search view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_statuses_count: Option<u64>,

    /// Extended trending/moderation metadata, present only in the trending/admin tag view (e.g. a
    /// group's `tags`) and absent from the lightweight tags inlined in a status.
    #[serde(flatten)]
    pub trending: Option<TagTrending>,
}

/// The extended trending/moderation fields of a [`Tag`].
///
/// These fields always appear together (in the trending/admin view), so they are grouped into one
/// struct that is flattened into [`Tag`] behind a single `Option` rather than making each field
/// individually optional.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TagTrending {
    /// Numeric hashtag id.
    pub id: u64,

    /// When the hashtag was first seen.
    pub created_at: DateTime<Utc>,

    /// When the hashtag's metadata was last updated.
    pub updated_at: DateTime<Utc>,

    /// When the hashtag was most recently used.
    pub last_status_at: DateTime<Utc>,

    /// Whether the hashtag may be used.
    pub usable: bool,

    /// Whether the hashtag is allowed to trend.
    pub trendable: bool,

    /// Whether the hashtag is listed publicly.
    pub listable: bool,

    /// When the hashtag was reviewed by a moderator, if ever.
    pub reviewed_at: Option<DateTime<Utc>>,

    /// When review of the hashtag was requested, if ever.
    pub requested_review_at: Option<DateTime<Utc>>,

    /// The hashtag's peak trending score, if scored (a number; `serde_json::Number` keeps
    /// [`TagTrending`] `Eq`/`Hash`).
    pub max_score: Option<serde_json::Number>,

    /// When the hashtag reached its peak trending score, if ever.
    pub max_score_at: Option<DateTime<Utc>>,
}

/// A single day's usage bucket in a [`Tag`]'s [`history`](Tag::history).
///
/// The API sends the counts as decimal strings (Mastodon convention), so `uses` and `accounts` are
/// decoded through `integer_str` while `days_ago` arrives as a plain number.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TagHistory<'a> {
    /// Start of the day this bucket covers, as a Unix timestamp (seconds) string.
    pub day: Cow<'a, str>,

    /// Number of uses of the hashtag that day.
    #[serde(with = "integer_str")]
    pub uses: u64,

    /// Number of distinct accounts that used the hashtag that day.
    #[serde(with = "integer_str")]
    pub accounts: u64,

    /// How many days before the response this bucket is (`0` is the current day). Truth Social
    /// specific.
    pub days_ago: u64,
}

/// A preview card for linked content.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Card<'a> {
    /// Unique identifier for this card, if any.
    pub id: Option<u64>,

    /// URL of the linked resource.
    pub url: Cow<'a, str>,

    /// Title of the linked resource.
    pub title: Cow<'a, str>,

    /// Description of the linked resource.
    pub description: Cow<'a, str>,

    /// Type of preview card.
    #[serde(rename = "type")]
    pub card_type: CardType,

    /// Author name, if available.
    pub author_name: Option<Cow<'a, str>>,

    /// Author URL, if available.
    pub author_url: Option<Cow<'a, str>>,

    /// Provider name (e.g., `YouTube`).
    pub provider_name: Option<Cow<'a, str>>,

    /// Provider URL.
    pub provider_url: Option<Cow<'a, str>>,

    /// HTML for embedding, if available.
    pub html: Option<Cow<'a, str>>,

    /// Suggested width for embed.
    pub width: Option<u64>,

    /// Suggested height for embed.
    pub height: Option<u64>,

    /// Preview image URL.
    pub image: Option<Cow<'a, str>>,

    /// Blurhash for preview image.
    pub blurhash: Option<Cow<'a, str>>,

    /// URL for embedding, if different from main URL.
    pub embed_url: Option<Cow<'a, str>>,

    /// Associated links, if any.
    pub links: Option<serde_json::Value>,

    /// Associated group, if any.
    #[serde(rename = "group")]
    pub card_group: Option<serde_json::Value>,
}

/// Type of preview card.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CardType {
    /// Generic link preview.
    Link,
    /// Photo preview.
    Photo,
    /// Video preview.
    Video,
    /// Rich embed (e.g., `YouTube`).
    Rich,
}

/// A group on Truth Social.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group<'a> {
    /// Unique identifier for this group.
    #[serde(with = "integer_str")]
    pub id: u64,

    /// Display name of the group.
    pub display_name: Cow<'a, str>,

    /// URL slug for the group.
    pub slug: Option<Cow<'a, str>>,

    /// Whether this is a private group.
    pub locked: Option<bool>,

    /// Whether this group is discoverable in search.
    pub discoverable: Option<bool>,

    /// Group avatar URL.
    pub avatar: Option<Cow<'a, str>>,

    /// Static version of the group avatar URL.
    pub avatar_static: Option<Cow<'a, str>>,

    /// Group header/banner URL.
    pub header: Option<Cow<'a, str>>,

    /// Static version of the group header URL.
    pub header_static: Option<Cow<'a, str>>,

    /// Group description (HTML formatted).
    pub note: Option<Cow<'a, str>>,

    /// Group visibility setting.
    pub group_visibility: Option<Cow<'a, str>>,

    /// When this group was created.
    pub created_at: Option<DateTime<Utc>>,

    /// When this group was deleted, if it has been.
    pub deleted_at: Option<DateTime<Utc>>,

    /// Whether membership is required to view content.
    pub membership_required: Option<bool>,

    /// Number of members in the group.
    pub members_count: Option<u64>,

    /// Tags associated with this group.
    pub tags: Option<Vec<Tag<'a>>>,

    /// URL to the group's page.
    pub url: Option<Cow<'a, str>>,

    /// Source content for the group (raw text).
    pub source: Option<GroupSource<'a>>,

    /// Owner of the group.
    pub owner: Option<GroupOwner>,

    /// Event category name, if this group is an event (Truth Social specific). Absent in older
    /// snapshots.
    #[serde(default)]
    pub event_category_name: Option<Cow<'a, str>>,

    /// Event subcategory name, if this group is an event (Truth Social specific). Absent in older
    /// snapshots.
    #[serde(default)]
    pub event_subcategory_name: Option<Cow<'a, str>>,

    /// Search-relevance rank, present only when this group is returned by `GET /api/v2/search`
    /// (`type=groups`) and absent when the group is embedded in a status.
    #[serde(default)]
    pub position: Option<u64>,

    /// The searching user's membership details, present only in group search results. Observed as
    /// `null` for every group in the captured fixtures (the session was not a member), so the inner
    /// shape is left unmodeled: a non-null value would fail to deserialize and surface the real
    /// shape rather than being silently accepted.
    #[serde(default)]
    pub member_details: Option<()>,

    /// A small preview sample of member avatars (Truth Social specific), present in the single-group
    /// lookup response (`GET /api/v1/groups/:id`) and absent when the group is embedded in a status
    /// or returned by search.
    #[serde(default)]
    pub member_avatars: Option<Vec<GroupMemberAvatar<'a>>>,
}

/// A member preview in a [`Group`]'s [`member_avatars`](Group::member_avatars): the lightweight
/// account fields needed to render an avatar stack.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMemberAvatar<'a> {
    /// Account id of the member.
    #[serde(with = "integer_str")]
    pub id: u64,

    /// Member's handle (without a leading `@`).
    pub username: Cow<'a, str>,

    /// Member's avatar URL.
    pub avatar: Cow<'a, str>,

    /// Static version of the member's avatar URL.
    pub avatar_static: Cow<'a, str>,
}

/// Source content for a group.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroupSource<'a> {
    /// Raw note/description text (not HTML).
    pub note: Cow<'a, str>,
}

/// Owner information for a group.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroupOwner {
    /// Account ID of the group owner.
    #[serde(with = "integer_str")]
    pub id: u64,
}

/// A member's role within a group, as reported by `GET /api/v1/groups/:id/memberships`.
///
/// The wire form is lowercase; note that a regular member serializes as `"user"` (not `"member"`),
/// matching the endpoint's `role` query parameter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupMembershipRole {
    /// The group's owner.
    Owner,
    /// A group administrator.
    Admin,
    /// A regular member (wire value `"user"`).
    User,
}

/// A single entry from `GET /api/v1/groups/:id/memberships`.
///
/// Pairs a member's [`account`](Self::account) with their [`role`](Self::role) in the group. The
/// endpoint returns a bare array of these, so no wrapper type is needed.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMembership<'a> {
    /// The member's role in the group.
    pub role: GroupMembershipRole,

    /// Membership id (distinct from the account id), string-encoded on the wire.
    #[serde(with = "integer_str")]
    pub id: u64,

    /// The member's full account.
    pub account: Account<'a>,
}

/// A poll attached to a status.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Poll<'a> {
    /// Unique identifier for this poll.
    #[serde(with = "integer_str")]
    pub id: u64,

    /// When the poll expires.
    pub expires_at: Option<DateTime<Utc>>,

    /// Whether the poll has expired.
    pub expired: bool,

    /// Whether multiple choices are allowed.
    pub multiple: bool,

    /// Total number of votes cast.
    pub votes_count: u64,

    /// Total number of unique voters.
    pub voters_count: Option<u64>,

    /// Whether the current user has voted.
    pub voted: Option<bool>,

    /// Indices of options the current user voted for.
    pub own_votes: Option<Vec<u64>>,

    /// The poll options.
    pub options: Vec<PollOption<'a>>,

    /// Custom emoji used in poll options.
    pub emojis: Vec<Emoji<'a>>,
}

/// An option in a poll.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PollOption<'a> {
    /// The option text.
    pub title: Cow<'a, str>,

    /// Number of votes for this option.
    pub votes_count: Option<u64>,
}

/// A custom emoji.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Emoji<'a> {
    /// Shortcode for the emoji (without colons).
    pub shortcode: Cow<'a, str>,

    /// URL to the emoji image.
    pub url: Cow<'a, str>,

    /// URL to the static version of the emoji.
    pub static_url: Cow<'a, str>,

    /// Whether this emoji is visible in the picker.
    pub visible_in_picker: bool,

    /// Category for organizing emoji.
    pub category: Option<Cow<'a, str>>,
}

/// A profile metadata field.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Field<'a> {
    /// Field label/name.
    pub name: Cow<'a, str>,

    /// Field value (may contain HTML links).
    pub value: Cow<'a, str>,

    /// When this field was verified, if applicable.
    pub verified_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    // Status IDs are long opaque integers; digit separators do not aid readability here.
    #![allow(clippy::unreadable_literal)]

    use super::*;

    /// Decodes a curated snapshot fixture to its JSON text, transparently gunzipping payloads
    /// stored in their original gzip-compressed form (magic `1f 8b`). The curated snapshots under
    /// `tests/data/wbm/snapshots/` keep the origin's exact bytes, so some are compressed; this
    /// mirrors the decoding done by the `wbm_snapshots` integration test's fixture guard.
    fn decode_snapshot(bytes: &[u8]) -> String {
        use std::io::Read as _;
        if bytes.starts_with(&[0x1f, 0x8b]) {
            let mut text = String::new();
            flate2::read::GzDecoder::new(bytes)
                .read_to_string(&mut text)
                .expect("gzip fixture decompresses to text");
            text
        } else {
            String::from_utf8(bytes.to_vec()).expect("fixture is UTF-8 JSON")
        }
    }

    /// `last_status_at` accepts a plain date ([`Either::Left`]), a full RFC 3339 datetime
    /// ([`Either::Right`]), and `null` (Truth Social returns all three), preserving each form and
    /// round-tripping back to the same text.
    #[test]
    fn last_status_at_accepts_date_or_datetime() {
        #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
        struct Wrapper {
            #[serde(default, with = "either::serde_untagged_optional")]
            value: Option<Either<NaiveDate, DateTime<Utc>>>,
        }

        let date = NaiveDate::from_ymd_opt(2023, 5, 23).unwrap();
        let datetime = "2023-05-23T22:31:45.360Z".parse::<DateTime<Utc>>().unwrap();
        let cases = [
            (r#"{"value":"2023-05-23"}"#, Some(Either::Left(date))),
            (
                r#"{"value":"2023-05-23T22:31:45.360Z"}"#,
                Some(Either::Right(datetime)),
            ),
            (r#"{"value":null}"#, None),
        ];
        for (json, expected) in cases {
            let parsed: Wrapper = serde_json::from_str(json).expect("deserializes");
            assert_eq!(parsed.value, expected, "for {json}");
            assert_eq!(
                serde_json::to_string(&parsed).unwrap(),
                json,
                "round-trip {json}"
            );
        }

        assert!(serde_json::from_str::<Wrapper>(r#"{"value":"not a date"}"#).is_err());
    }

    /// A lightweight status tag (`{name, url}`) carries no [`TagTrending`]; a full trending-view
    /// tag flattens its extra fields into one. Both round-trip; the absent block emits no keys.
    #[test]
    fn tag_trending_flattens_optionally() {
        let lightweight: Tag<'_> =
            serde_json::from_str(r#"{"name":"Truth","url":"https://truthsocial.com/tags/Truth"}"#)
                .expect("lightweight tag");
        assert!(lightweight.trending.is_none());
        assert_eq!(
            serde_json::to_value(&lightweight).unwrap(),
            serde_json::json!({"name": "Truth", "url": "https://truthsocial.com/tags/Truth"}),
        );

        let full_json = r#"{"name":"music","url":null,"id":227,"created_at":"2022-02-16T22:38:42.721Z","updated_at":"2023-02-17T15:08:07.243Z","last_status_at":"2023-02-17T15:08:06.953Z","usable":true,"trendable":true,"listable":true,"reviewed_at":null,"requested_review_at":null,"max_score":null,"max_score_at":null}"#;
        let full: Tag<'_> = serde_json::from_str(full_json).expect("full tag");
        let trending = full.trending.as_ref().expect("trending present");
        assert_eq!(trending.id, 227);
        assert!(trending.usable && trending.trendable && trending.listable);
        assert!(trending.max_score.is_none());
        assert_eq!(
            serde_json::to_value(&full).unwrap(),
            serde_json::from_str::<serde_json::Value>(full_json).unwrap(),
        );
    }

    /// A reblog (boost): the outer status carries a `reblog` with its own account and card.
    #[test]
    fn test_deserialize_reblog() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AAA7WWHWJB6AP4HLEMQBVJQP6QSDQV4Q"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 107962811324118694);
        assert_eq!(status.visibility, Some(Visibility::Public));
        assert_eq!(status.account.username, "DineshDSouza");

        let reblog = status.reblog.as_ref().expect("Expected reblog");
        assert_eq!(reblog.id, 107957941707714512);
        assert_eq!(reblog.account.username, "DineshDSouza");
        assert!(reblog.account.verified);

        // Card on the reblogged status.
        let card = reblog.card.as_ref().expect("Expected card");
        assert_eq!(card.card_type, CardType::Link);
        assert_eq!(card.provider_name.as_deref(), Some("dinesh.locals.com"));
    }

    /// A video attachment with full `original` and `small` metadata, plus the newer
    /// `premium`/`version`/`editable` status fields.
    #[test]
    fn test_deserialize_video_metadata() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AASQSDZXOZ2C7Z2UMTZTIJ63I33KY6A3"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 116616050796377967);
        assert_eq!(status.account.username, "OAN");
        assert_eq!(status.account.premium, Some(true));

        // Newer Status fields.
        assert_eq!(status.version.as_deref(), Some("1"));
        assert_eq!(status.editable, Some(false));
        assert!(status.edited_at.is_none());

        assert_eq!(status.media_attachments.len(), 1);
        let attachment = &status.media_attachments[0];
        assert_eq!(attachment.media_type, MediaType::Video);
        assert_eq!(attachment.processing, Some(ProcessingStatus::Complete));
        assert!(attachment.blurhash.is_some());

        let meta = &attachment.meta;
        let original = meta.original.as_ref().expect("Expected original metadata");
        assert_eq!(original.bitrate, Some(4323756));
        assert!(original.duration.is_some());
        assert!(original.frame_rate.is_some());

        let small = meta
            .small
            .as_ref()
            .expect("Expected small thumbnail metadata");
        assert_eq!(small.width, Some(1067));
        assert_eq!(small.height, Some(600));
    }

    /// The model must deserialize from an owned [`serde_json::Value`] tree, not only from a
    /// borrowable `&str` or reader. `serde_json::from_value` cannot hand out borrows into the tree,
    /// so any field typed as a plain `&'de str` (rather than [`Cow`]) would fail here with
    /// "invalid type: string, expected a borrowed string" the moment that field is populated.
    ///
    /// This guards against such a field silently regressing the `from_value` path: it reuses the
    /// widest real payload (account extensions, media, nested meta, card) and round-trips it through
    /// `to_value` then `from_value`. `from_str` alone would not catch the regression, since a
    /// borrowed field parses fine there.
    #[test]
    fn status_deserializes_from_owned_value() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AASQSDZXOZ2C7Z2UMTZTIJ63I33KY6A3"
        ));
        let value: serde_json::Value = serde_json::from_str(&json).expect("snapshot is valid JSON");
        let from_value: Status<'_> = serde_json::from_value(value)
            .expect("Status must deserialize from an owned Value tree");

        // Re-encoding an owned parse and a borrowed parse must agree, confirming no field was
        // dropped or altered on the non-borrowing path.
        let from_str: Status<'_> = serde_json::from_str(&json).expect("borrowed parse");
        assert_eq!(
            serde_json::to_value(&from_value).unwrap(),
            serde_json::to_value(&from_str).unwrap(),
        );
    }

    /// A status whose `visibility` is JSON `null` must deserialize to `None`, not fail. Some older
    /// captures (and Wayback Machine snapshots) serialize the field this way; the field is required
    /// on the live API but optional in the archive.
    #[test]
    fn status_accepts_null_visibility() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AAA7WWHWJB6AP4HLEMQBVJQP6QSDQV4Q"
        ));
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("snapshot JSON");
        // Force the top-level visibility to null, mirroring the real captures that triggered this.
        value["visibility"] = serde_json::Value::Null;

        let status: Status<'_> =
            serde_json::from_value(value).expect("a null visibility must deserialize, not error");
        assert_eq!(status.visibility, None);
    }

    /// A status whose account carries the `pleroma` extension and whose media is a video
    /// backed by a `CardType::Video` link card (older capture: no blurhash, no processing).
    #[test]
    fn test_deserialize_pleroma_and_video_card() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AFOLBGJSXEG7R5OK2IFC24A7NDRITY7E"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 109967811792793526);
        assert_eq!(status.account.username, "realDonaldTrump");
        assert!(status.account.pleroma.is_some());

        assert_eq!(status.media_attachments.len(), 1);
        let attachment = &status.media_attachments[0];
        assert_eq!(attachment.media_type, MediaType::Video);
        assert!(attachment.blurhash.is_none());

        let meta = &attachment.meta;
        let original = meta.original.as_ref().expect("Expected original metadata");
        assert_eq!(original.bitrate, Some(1276287));
        assert!(original.duration.is_some());
        assert!(original.frame_rate.is_some());

        let card = status.card.as_ref().expect("Expected card");
        assert_eq!(card.card_type, CardType::Video);
        assert!(card.id.is_none()); // Card ID can be null.
        assert_eq!(card.provider_name.as_deref(), Some("Rumble.com"));
    }

    /// A quote post: `quote_id` plus the embedded `quote` status (which itself has a video card).
    #[test]
    fn test_deserialize_quote() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AAINYHXV73DIWMOPHE7372JKHHSPJBAV"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 110375104524907659);
        assert_eq!(status.account.username, "ChrisSalcedoShow");
        assert_eq!(status.quote_id, Some(110374797872162426));

        let quote = status.quote.as_ref().expect("Expected quote");
        assert_eq!(quote.id, 110374797872162426);
        assert_eq!(quote.account.username, "DevinNunes");
        assert_eq!(quote.visibility, Some(Visibility::Public));

        let card = quote.card.as_ref().expect("Expected card on quote");
        assert_eq!(card.card_type, CardType::Video);
    }

    /// A status with a suspended account in reply (tombstoned, deleted).
    #[test]
    fn test_deserialize_suspended_account() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/LR2C2SGUH5OHC7IZCTO7R54W3MQTHOKO"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        // Verify top-level status
        assert_eq!(status.id, 107838474127241768);
        assert_eq!(status.account.username, "trumpcash47");
        assert!(status.account.verified);

        // Verify `in_reply_to` has a suspended account
        let reply = status.in_reply_to.as_ref().expect("Expected in_reply_to");
        assert_eq!(reply.id, 107838404966019561);
        assert_eq!(reply.account.username, "samanthamarika");
        assert_eq!(reply.account.suspended, Some(true));

        // Verify the reply has a tombstone (deleted)
        let tombstone = reply.tombstone.as_ref().expect("Expected tombstone");
        assert_eq!(tombstone.reason, TombstoneReason::Deleted);
        assert!(reply.deleted_at.is_some());
    }

    /// A reply to a deleted status: the `in_reply_to` is tombstoned with placeholder content,
    /// while the top-level status is intact.
    #[test]
    fn test_deserialize_deleted_reply() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/ADDTCWFH37EYIWPVHC7LUXQ6NND5YAZM"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 110651508324557686);
        assert_eq!(status.account.username, "BrianCates");
        assert!(status.tombstone.is_none()); // Top-level is not deleted.

        let reply = status.in_reply_to.as_ref().expect("Expected in_reply_to");
        assert_eq!(reply.id, 110651505508533166);
        assert_eq!(reply.account.username, "BrianCates");

        let tombstone = reply.tombstone.as_ref().expect("Expected tombstone");
        assert_eq!(tombstone.reason, TombstoneReason::Deleted);
        assert!(reply.deleted_at.is_some());
        assert_eq!(reply.content, "Unavailable.");
    }

    /// An older-format capture: newer optional status/account fields are absent and so
    /// deserialize to `None`.
    #[test]
    fn test_deserialize_older_format() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/ACIOAULA7PEKDUZPYMJHCW3OSVWW6OKY"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 108742189401812078);
        assert_eq!(status.account.username, "realDonaldTrump");

        // Newer status fields are absent in this older capture.
        assert!(status.sponsored.is_none());
        assert!(status.upvotes_count.is_none());
        assert!(status.downvotes_count.is_none());
        assert!(status.votable.is_none());

        // Newer account fields are likewise absent.
        assert!(status.account.unauth_visibility.is_none());
        assert!(status.account.tv_onboarded.is_none());
    }

    /// A sponsored status: `sponsored`, `metrics`, and `Visibility::Unlisted`.
    #[test]
    fn test_deserialize_sponsored_with_metrics() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AJ5CMWDL6TXYHJOJ4E56WMOXDJSTPNJQ"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 110941119092973173);
        assert_eq!(status.account.username, "epochtimes");
        assert_eq!(status.visibility, Some(Visibility::Unlisted));
        assert_eq!(status.sponsored, Some(true));

        let metrics = status.metrics.as_ref().expect("Expected metrics");
        assert!(metrics.impression.is_some());
        assert!(metrics.expires_at.is_some());
    }

    /// A group post: `Visibility::Group` plus the embedded `group` with its extended metadata,
    /// source, and owner.
    #[test]
    fn test_deserialize_group_post() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/ACEWLCRAJMERJRZS4UMDE36SPTSJ2UVP"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 110617267048515904);
        assert_eq!(status.account.username, "elenochle");
        assert_eq!(status.visibility, Some(Visibility::Group));

        let group = status.group.as_ref().expect("Expected group");
        assert_eq!(group.id, 110353664589615240);
        assert_eq!(group.display_name, "News Blast Links");
        assert_eq!(group.locked, Some(false));
        assert_eq!(group.discoverable, Some(true));
        assert_eq!(group.membership_required, Some(true));
        assert_eq!(group.members_count, Some(3106));

        assert!(group.avatar_static.is_some());
        assert!(group.header_static.is_some());
        assert!(group.source.is_some());

        let owner = group.owner.as_ref().expect("Expected group owner");
        assert_eq!(owner.id, 108318222345080429);
    }

    /// A TV status: `status.tv` channel metadata, a `tv_account`, and a `MediaType::Tv` attachment
    /// carrying its own TV metadata.
    #[test]
    fn test_deserialize_tv_status() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AAGCDRPMZ65KGPCTAGSZZMJCDHKKR3HA"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 115806398111692077);
        assert_eq!(status.account.username, "NewsMax");
        assert_eq!(status.account.tv_account, Some(true));

        let tv = status.tv.as_ref().expect("Expected tv metadata on status");
        assert_eq!(tv.channel_id, 5);
        assert_eq!(tv.name, "Newsfront S1:E4 – Marxism in America");
        assert_eq!(tv.pltv_timespan, 259200000);

        assert_eq!(status.media_attachments.len(), 1);
        let attachment = &status.media_attachments[0];
        assert_eq!(attachment.media_type, MediaType::Tv);
        assert_eq!(attachment.processing, Some(ProcessingStatus::Complete));
        assert!(attachment.tv.is_some());
    }

    /// A two-level reply chain: `in_reply_to` nested inside `in_reply_to`.
    #[test]
    fn test_deserialize_reply_chain() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AARDSO2IWOC37RSCMF66KUVLXHOS2OI6"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 115396476953854230);
        assert_eq!(status.account.username, "redneck");

        let reply1 = status.in_reply_to.as_ref().expect("Expected in_reply_to");
        assert_eq!(reply1.id, 115396468279137711);

        let reply2 = reply1
            .in_reply_to
            .as_ref()
            .expect("Expected nested in_reply_to");
        assert_eq!(reply2.id, 115396458435977605);
        assert_eq!(reply2.visibility, Some(Visibility::Public));
    }

    /// A status with many mentions (each an account reference) and an image attachment.
    #[test]
    fn test_deserialize_many_mentions() {
        let json = decode_snapshot(include_bytes!(
            "../../tests/data/wbm/snapshots/AANYHQREUL55MLX5HNJ2LEEZZXXL7NDY"
        ));
        let status: Status<'_> = serde_json::from_str(&json).expect("Failed to deserialize status");

        assert_eq!(status.id, 108407676531874529);
        assert_eq!(status.account.username, "RatDog2020");

        assert_eq!(status.mentions.len(), 15);
        // Each mention resolves to an account reference.
        assert!(matches!(status.mentions[0].id, MentionId::Account(_)));

        assert_eq!(status.media_attachments.len(), 1);
        let attachment = &status.media_attachments[0];
        assert_eq!(attachment.media_type, MediaType::Image);
        assert_eq!(attachment.processing, Some(ProcessingStatus::Complete));
        assert!(attachment.blurhash.is_some());
    }

    /// The `Visibility` variants that do not appear in the curated snapshot corpus are covered by
    /// parsing their wire representations directly.
    #[test]
    fn test_visibility_wire_variants() {
        assert_eq!(
            serde_json::from_str::<Visibility>("\"self\"").unwrap(),
            Visibility::Self_
        );
        assert_eq!(
            serde_json::from_str::<Visibility>("\"private\"").unwrap(),
            Visibility::Private
        );
        assert_eq!(
            serde_json::from_str::<Visibility>("\"direct\"").unwrap(),
            Visibility::Direct
        );
    }

    /// Builds a minimal but complete synthetic [`Account`] JSON: every field Truth Social always
    /// sends (the `Option<_>` fields are omitted and default to `None`), with placeholder handles so
    /// no real user data appears in tests. Shaped like the account objects under `tests/data/`.
    fn synthetic_account_json(id: u64, username: &str) -> String {
        format!(
            r#"{{"id":"{id}","username":"{username}","acct":"{username}",
                "display_name":"Synthetic {username}","locked":false,"bot":false,"group":false,
                "created_at":"2024-01-01T00:00:00.000Z","note":"<p></p>",
                "url":"https://truthsocial.com/@{username}","avatar":"https://example.test/a.png",
                "avatar_static":"https://example.test/a.png","header":"https://example.test/h.png",
                "header_static":"https://example.test/h.png","followers_count":0,
                "following_count":0,"statuses_count":0,"verified":false,"location":"","website":"",
                "emojis":[],"fields":[]}}"#
        )
    }

    /// A `GET /api/v1/groups/:id/memberships` payload is a bare array of [`GroupMembership`]; each
    /// entry decodes its role, its string-encoded membership id (distinct from the account id), and
    /// the nested [`Account`]. Parsing and re-parsing the serialized form yields an equal value.
    #[test]
    fn group_memberships_parse_role_id_and_account() {
        let json = format!(
            r#"[{{"role":"owner","id":"1001","account":{owner}}},
                {{"role":"admin","id":"1002","account":{admin}}},
                {{"role":"user","id":"1003","account":{member}}}]"#,
            owner = synthetic_account_json(11, "placeholder_owner"),
            admin = synthetic_account_json(12, "placeholder_admin"),
            member = synthetic_account_json(13, "placeholder_member"),
        );

        let members = serde_json::from_str::<Vec<GroupMembership<'_>>>(&json)
            .expect("memberships deserialize");

        assert_eq!(
            members.iter().map(|m| m.role).collect::<Vec<_>>(),
            [
                GroupMembershipRole::Owner,
                GroupMembershipRole::Admin,
                GroupMembershipRole::User,
            ]
        );
        // Membership ids (`integer_str`-decoded) are distinct from the members' account ids.
        assert_eq!(
            members.iter().map(|m| m.id).collect::<Vec<_>>(),
            [1001, 1002, 1003]
        );
        assert_eq!(
            members.iter().map(|m| m.account.id).collect::<Vec<_>>(),
            [11, 12, 13]
        );
        assert_eq!(members[2].account.username, "placeholder_member");

        // Semantic round-trip: `Account` re-emits its optional fields as `null`, so the bytes differ,
        // but reparsing the serialized form must reproduce the same value.
        let reparsed = serde_json::from_str::<Vec<GroupMembership<'_>>>(
            &serde_json::to_string(&members).unwrap(),
        )
        .expect("serialized memberships reparse");
        assert_eq!(reparsed, members);
    }

    /// The membership `role` uses lowercase wire values and round-trips. Note that a regular member
    /// is `"user"`, not the UI term `"member"`. An unknown role and any extra key both fail loudly,
    /// upholding the strict deserialization contract.
    #[test]
    fn group_membership_role_wire_and_strictness() {
        for (wire, role) in [
            ("owner", GroupMembershipRole::Owner),
            ("admin", GroupMembershipRole::Admin),
            ("user", GroupMembershipRole::User),
        ] {
            let quoted = format!("\"{wire}\"");
            assert_eq!(
                serde_json::from_str::<GroupMembershipRole>(&quoted).unwrap(),
                role
            );
            assert_eq!(serde_json::to_string(&role).unwrap(), quoted);
        }

        // "member" is the UI vocabulary, not the wire value, so it must not parse.
        assert!(serde_json::from_str::<GroupMembershipRole>("\"member\"").is_err());

        // `deny_unknown_fields` rejects any key beyond {role,id,account}, even with a valid account.
        let account = synthetic_account_json(1, "placeholder");
        let valid = format!(r#"{{"role":"user","id":"1","account":{account}}}"#);
        assert!(serde_json::from_str::<GroupMembership<'_>>(&valid).is_ok());
        let extra = format!(r#"{{"role":"user","id":"1","account":{account},"unexpected":true}}"#);
        assert!(serde_json::from_str::<GroupMembership<'_>>(&extra).is_err());
    }

    /// A `MentionId` must deserialize identically whether or not the deserializer can borrow the
    /// input string. `from_str` can borrow, but owned/streaming deserializers (`from_reader`,
    /// `from_value`) cannot; all three must yield the same value, and mentions are ubiquitous in
    /// real statuses, so a borrow-only implementation would break common downstream code paths.
    #[test]
    fn mention_id_parses_in_owned_and_borrowed_modes() {
        let expected = MentionId::Account(12345);
        assert_eq!(
            serde_json::from_str::<MentionId>("\"12345\"").unwrap(),
            expected
        );
        assert_eq!(
            serde_json::from_reader::<_, MentionId>(b"\"12345\"".as_slice()).unwrap(),
            expected
        );
        assert_eq!(
            serde_json::from_value::<MentionId>(serde_json::json!("12345")).unwrap(),
            expected
        );
        // The domain sentinel resolves the same way in a non-borrowing deserializer.
        assert_eq!(
            serde_json::from_reader::<_, MentionId>(b"\"-99\"".as_slice()).unwrap(),
            MentionId::Domain
        );
    }
}
