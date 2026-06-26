//! View models shared across TUI screens.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Identifies a UI screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Screen {
    /// Browse the feed for the selected category.
    Feed,
    /// Compose a new post.
    Compose,
    /// View a post and its comment thread.
    Thread,
    /// Manage category subscriptions.
    Subscriptions,
    /// Show peer and sync status.
    SyncStatus,
    /// Show key management status and actions.
    KeyManagement,
}

/// Summary of a category object suitable for the UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CategorySummary {
    /// Category object ID.
    pub object_id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Short description.
    pub description: String,
    /// Category scope identifier.
    pub category_id: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Summary of a post for the feed list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedPost {
    /// Post object ID.
    pub object_id: String,
    /// Author public key hex.
    pub author_id: String,
    /// Post text body.
    pub text: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// A node in the comment tree under a post.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadComment {
    /// Comment object ID.
    pub object_id: String,
    /// Author public key hex.
    pub author_id: String,
    /// Comment text body.
    pub text: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Direct replies to this comment.
    pub replies: Vec<Self>,
    /// Whether this branch is collapsed in the UI.
    pub collapsed: bool,
}

/// Thread view model for a selected post.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadView {
    /// Root post.
    pub post: FeedPost,
    /// Top-level comments under the post.
    pub comments: Vec<ThreadComment>,
}

/// Local subscription record persisted by the TUI.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Subscriptions {
    /// Category IDs the user has subscribed to locally.
    pub category_ids: Vec<String>,
}

/// Peer status as shown in the sync panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerStatus {
    /// Optional display name.
    pub name: Option<String>,
    /// Peer HTTP endpoint.
    pub address: String,
    /// Whether the last sync to this peer succeeded.
    pub last_sync_ok: Option<bool>,
}

/// Result of a manual sync attempt.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SyncTotals {
    /// Objects pulled from peers.
    pub pulled: usize,
    /// Objects pushed to peers.
    pub pushed: usize,
    /// Objects rejected by a peer or local store.
    pub rejected: usize,
}

/// Key status shown in the key management panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyStatus {
    /// No key file exists; the user must generate one.
    Missing,
    /// A key exists; show its public identity and backup hints.
    Present {
        /// Hex-encoded Ed25519 public key.
        public_key_hex: String,
    },
}

/// A warning that a category or peer is seen for the first time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FirstSeenWarning {
    /// A category ID has not been seen before.
    Category {
        /// Category ID.
        category_id: String,
        /// Display name if known.
        display_name: Option<String>,
    },
    /// A peer address has not been seen before.
    Peer {
        /// Peer HTTP endpoint.
        address: String,
    },
}

/// Set of acknowledged first-seen values persisted by the TUI.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AcknowledgedFirstSeen {
    /// Acknowledged category IDs.
    pub categories: Vec<String>,
    /// Acknowledged peer addresses.
    pub peers: Vec<String>,
}
