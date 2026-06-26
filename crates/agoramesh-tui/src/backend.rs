//! Data gateway between the TUI and the underlying `AgoraMesh` crates.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use agoramesh_cli::config::Config;
use agoramesh_cli::keyring::{self, Keyring};
use agoramesh_cli::peers::Peers;
use agoramesh_core::SystemClock;
use agoramesh_core::identity::Keypair;
use agoramesh_core::objects::{
    ParentKind, category as category_obj, comment as comment_obj, post as post_obj,
};
use agoramesh_store::db::{Connection, SqliteStore};
use agoramesh_store::store::Store;
use chrono::{DateTime, Utc};

use crate::error::Error;
use crate::models::{
    AcknowledgedFirstSeen, CategorySummary, FeedPost, KeyStatus, PeerStatus, Subscriptions,
    ThreadComment, ThreadView,
};

const SUBSCRIPTIONS_FILE: &str = "subscriptions.json";
const FIRST_SEEN_FILE: &str = "seen.json";

/// Gateway that exposes TUI-friendly operations over a data directory.
#[derive(Debug)]
pub struct Backend {
    config: Config,
    plaintext: bool,
}

impl Backend {
    /// Opens the backend for the given data directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the data directory or store cannot be initialized.
    pub fn open(data_dir: Option<PathBuf>, plaintext: bool) -> Result<Self, Error> {
        let config = Config::open(data_dir)?;
        Ok(Self { config, plaintext })
    }

    /// Opens the `SQLite` store for this backend.
    ///
    /// Exposed publicly so integration tests and TUI event loops can read and
    /// write messages through the verified store.
    pub fn store(&self) -> Result<SqliteStore, Error> {
        let connection = Connection::open(&self.config.store_path())?;
        Ok(SqliteStore::new(connection))
    }

    /// Returns the filesystem path used by this backend.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }

    /// Returns the path to the local subscriptions file.
    fn subscriptions_path(&self) -> PathBuf {
        self.config.data_dir.join(SUBSCRIPTIONS_FILE)
    }

    /// Returns the path to the first-seen acknowledgements file.
    fn first_seen_path(&self) -> PathBuf {
        self.config.data_dir.join(FIRST_SEEN_FILE)
    }

    /// Loads locally persisted subscriptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub fn load_subscriptions(&self) -> Result<Subscriptions, Error> {
        load_json(&self.subscriptions_path(), Subscriptions::default())
    }

    /// Saves locally persisted subscriptions.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    pub fn save_subscriptions(&self, subscriptions: &Subscriptions) -> Result<(), Error> {
        save_json(&self.subscriptions_path(), subscriptions)
    }

    /// Loads acknowledged first-seen values.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or parsed.
    pub fn load_acknowledged(&self) -> Result<AcknowledgedFirstSeen, Error> {
        load_json(&self.first_seen_path(), AcknowledgedFirstSeen::default())
    }

    /// Saves acknowledged first-seen values.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    pub fn save_acknowledged(&self, acknowledged: &AcknowledgedFirstSeen) -> Result<(), Error> {
        save_json(&self.first_seen_path(), acknowledged)
    }

    /// Returns the current key status, generating a dev plaintext key when
    /// requested and `plaintext` mode is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error when the key file cannot be read or generated.
    pub fn key_status(&self, generate_if_missing: bool) -> Result<KeyStatus, Error> {
        let path = self.config.key_path();
        if !path.exists() && generate_if_missing {
            if self.plaintext {
                Keyring::new(&path).dev_plaintext_save()?;
            } else {
                return Ok(KeyStatus::Missing);
            }
        }
        if !path.exists() {
            return Ok(KeyStatus::Missing);
        }
        let keypair = self.load_keypair()?;
        Ok(KeyStatus::Present {
            public_key_hex: keyring::public_key_hex(&keypair),
        })
    }

    /// Generates a new development plaintext key for the configured data dir.
    ///
    /// # Errors
    ///
    /// Returns an error when the key file cannot be written.
    pub fn generate_dev_key(&self) -> Result<KeyStatus, Error> {
        if !self.plaintext {
            return Err(Error::Message(
                "plaintext key generation is only available in dev mode".to_owned(),
            ));
        }
        Keyring::new(&self.config.key_path()).dev_plaintext_save()?;
        self.key_status(false)
    }

    fn load_keypair(&self) -> Result<Keypair, Error> {
        if self.plaintext {
            return Ok(Keyring::new(&self.config.key_path()).dev_plaintext_load()?);
        }
        Err(Error::Message(
            "encrypted key loading is not yet supported by the TUI".to_owned(),
        ))
    }

    /// Loads categories stored in the local store, newest last.
    ///
    /// # Errors
    ///
    /// Returns an error when the store cannot be read or a message fails
    /// verification.
    pub fn load_categories(&self) -> Result<Vec<CategorySummary>, Error> {
        let store = self.store()?;
        let clock = SystemClock;
        let messages = store.list_by_type("category", &clock)?;
        let mut summaries = Vec::with_capacity(messages.len());
        for message in messages {
            let body: category_obj::Body = serde_json::from_slice(message.signed_payload().body())
                .map_err(|error| Error::Message(error.to_string()))?;
            summaries.push(CategorySummary {
                object_id: message.id().to_hex(),
                display_name: body.display_name,
                description: body.description,
                category_id: body.category_id,
                created_at: body.created_at,
            });
        }
        Ok(summaries)
    }

    /// Loads posts in the given category scope, oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error when the store cannot be read or a message fails
    /// verification.
    pub fn load_feed(&self, category_id: &str) -> Result<Vec<FeedPost>, Error> {
        let store = self.store()?;
        let clock = SystemClock;
        let messages = store.list_by_scope(category_id, &clock)?;
        let mut posts = Vec::new();
        for message in messages {
            if message.signed_payload().kind() != "post" {
                continue;
            }
            let body: post_obj::Body = serde_json::from_slice(message.signed_payload().body())
                .map_err(|error| Error::Message(error.to_string()))?;
            posts.push(FeedPost {
                object_id: message.id().to_hex(),
                author_id: hex::encode(message.signed_payload().author_pubkey()),
                text: body.text,
                created_at: body.created_at,
            });
        }
        Ok(posts)
    }

    /// Loads the thread view for a post, including its comment tree.
    ///
    /// # Errors
    ///
    /// Returns an error when the store cannot be read, the post is missing, or
    /// a message fails verification.
    pub fn load_thread(&self, post_id: &str) -> Result<ThreadView, Error> {
        let store = self.store()?;
        let clock = SystemClock;
        let post_message_id = agoramesh_core::MessageId::from_hex(post_id)
            .map_err(|error| Error::Message(error.to_string()))?;
        let post_message = store
            .get(post_message_id, &clock)?
            .ok_or_else(|| Error::Message(format!("post {post_id} not found")))?;
        let post_body: post_obj::Body =
            serde_json::from_slice(post_message.signed_payload().body())
                .map_err(|error| Error::Message(error.to_string()))?;
        let post = FeedPost {
            object_id: post_message.id().to_hex(),
            author_id: hex::encode(post_message.signed_payload().author_pubkey()),
            text: post_body.text,
            created_at: post_body.created_at,
        };

        let scope = post_message.signed_payload().scope();
        let messages = store.list_by_scope(scope, &clock)?;
        let mut comments_by_parent = HashMap::new();
        for message in messages {
            if message.signed_payload().kind() != "comment" {
                continue;
            }
            let loaded_comment = LoadedComment::from_message(&message)?;
            comments_by_parent
                .entry(loaded_comment.parent_id)
                .or_insert_with(Vec::new)
                .push(loaded_comment);
        }
        let comments =
            build_comment_tree(post_message_id, &ParentKind::Post, &mut comments_by_parent);

        Ok(ThreadView { post, comments })
    }

    /// Creates a signed post and inserts it into the local store.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is missing, validation fails, or persistence
    /// fails.
    pub fn create_post(
        &self,
        category_id: &str,
        text: &str,
        created_at: DateTime<Utc>,
    ) -> Result<FeedPost, Error> {
        let keypair = self.load_keypair()?;
        let message = post_obj::create(&keypair, category_id, text, created_at)?;
        let object_id = message.id().to_hex();
        let author_id = hex::encode(message.signed_payload().author_pubkey());
        let mut store = self.store()?;
        let clock = SystemClock;
        agoramesh_core::objects::validation::validate_phase1_message(&message)
            .map_err(Error::from)?;
        store.insert(message, &clock)?;
        Ok(FeedPost {
            object_id,
            author_id,
            text: text.to_owned(),
            created_at,
        })
    }

    /// Loads peer statuses from the persisted peers file.
    ///
    /// # Errors
    ///
    /// Returns an error when the peers file cannot be read or parsed.
    pub fn peer_statuses(&self) -> Result<Vec<PeerStatus>, Error> {
        let peers = Peers::load(&self.config.peers_path())?;
        Ok(peers
            .list()
            .iter()
            .map(|peer| PeerStatus {
                name: peer.name.clone(),
                address: peer.address.clone(),
                last_sync_ok: None,
            })
            .collect())
    }
}

fn load_json<T: Default + serde::de::DeserializeOwned>(
    path: &Path,
    default: T,
) -> Result<T, Error> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(Error::StateJson),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(default),
        Err(source) => Err(Error::StateIo(source)),
    }
}

fn save_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::StateIo)?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(Error::StateJson)?;
    std::fs::write(path, bytes).map_err(Error::StateIo)
}

#[derive(Debug)]
struct LoadedComment {
    object_id: agoramesh_core::MessageId,
    object_id_hex: String,
    author_id: String,
    text: String,
    created_at: DateTime<Utc>,
    parent_kind: ParentKind,
    parent_id: agoramesh_core::MessageId,
}

impl LoadedComment {
    fn from_message(message: &agoramesh_core::Message) -> Result<Self, Error> {
        let body: comment_obj::Body = serde_json::from_slice(message.signed_payload().body())
            .map_err(|error| Error::Message(error.to_string()))?;
        let parent_id = agoramesh_core::MessageId::from_hex(&body.parent_id)
            .map_err(|error| Error::Message(error.to_string()))?;
        Ok(Self {
            object_id: message.id(),
            object_id_hex: message.id().to_hex(),
            author_id: hex::encode(message.signed_payload().author_pubkey()),
            text: body.text,
            created_at: body.created_at,
            parent_kind: body.parent_kind,
            parent_id,
        })
    }
}

fn sort_comments(comments: &mut [LoadedComment]) {
    comments.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.object_id_hex.cmp(&right.object_id_hex))
    });
}

fn build_comment_tree(
    parent_id: agoramesh_core::MessageId,
    parent_kind: &ParentKind,
    comments_by_parent: &mut HashMap<agoramesh_core::MessageId, Vec<LoadedComment>>,
) -> Vec<ThreadComment> {
    let mut comments = comments_by_parent.remove(&parent_id).unwrap_or_default();
    comments.retain(|comment| comment.parent_kind == *parent_kind);
    sort_comments(&mut comments);

    comments
        .into_iter()
        .map(|comment| ThreadComment {
            object_id: comment.object_id_hex,
            author_id: comment.author_id,
            text: comment.text,
            created_at: comment.created_at,
            replies: build_comment_tree(
                comment.object_id,
                &ParentKind::Comment,
                comments_by_parent,
            ),
            collapsed: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agoramesh_core::Keypair;
    use agoramesh_core::objects::{ParentKind, category, comment as comment_obj};
    use chrono::Timelike;

    fn backend_fixture(plaintext: bool) -> (Backend, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let backend =
            Backend::open(Some(temp_dir.path().to_path_buf()), plaintext).expect("open backend");
        (backend, temp_dir)
    }

    #[test]
    fn backend_loads_feed_from_sqlite_scope() {
        let (backend, _temp_dir) = backend_fixture(true);
        let keypair = Keypair::generate();
        let now = Utc::now();
        let created_at = now.with_nanosecond(0).expect("truncate to seconds");
        let category = category::create(
            &keypair,
            created_at,
            "Test Category",
            "A test category",
            "Initial charter text",
        )
        .expect("create category");
        let category_id = category.signed_payload().scope().to_owned();
        let mut store = backend.store().expect("open store");
        let clock = SystemClock;
        store.insert(category, &clock).expect("insert category");

        let post = post_obj::create(
            &keypair,
            &category_id,
            "Hello from the TUI feed",
            created_at,
        )
        .expect("create post");
        store.insert(post, &clock).expect("insert post");

        let posts = backend.load_feed(&category_id).expect("load feed");
        assert_eq!(posts.len(), 1);
        assert_eq!(
            posts.first().map_or("", |post| post.text.as_str()),
            "Hello from the TUI feed"
        );
    }

    #[test]
    fn backend_loads_nested_thread_replies() {
        let (backend, _temp_dir) = backend_fixture(true);
        let keypair = Keypair::generate();
        let created_at = Utc::now().with_nanosecond(0).expect("truncate to seconds");
        let category = category::create(
            &keypair,
            created_at,
            "Thread Category",
            "A test category",
            "Initial charter text",
        )
        .expect("create category");
        let category_id = category.signed_payload().scope().to_owned();
        let mut store = backend.store().expect("open store");
        let clock = SystemClock;
        store.insert(category, &clock).expect("insert category");

        let post = post_obj::create(
            &keypair,
            &category_id,
            "Hello from the thread view",
            created_at,
        )
        .expect("create post");
        let post_id = post.id().to_hex();
        store.insert(post, &clock).expect("insert post");

        let top_comment = comment_obj::create(
            &keypair,
            &category_id,
            ParentKind::Post,
            agoramesh_core::MessageId::from_hex(&post_id).expect("parse post id"),
            "Top-level comment",
            created_at,
        )
        .expect("create top comment");
        let top_comment_id = top_comment.id().to_hex();
        store
            .insert(top_comment, &clock)
            .expect("insert top comment");

        let first_reply = comment_obj::create(
            &keypair,
            &category_id,
            ParentKind::Comment,
            agoramesh_core::MessageId::from_hex(&top_comment_id).expect("parse top comment id"),
            "First reply",
            created_at,
        )
        .expect("create first reply");
        let first_reply_id = first_reply.id().to_hex();
        store
            .insert(first_reply, &clock)
            .expect("insert first reply");

        let second_reply = comment_obj::create(
            &keypair,
            &category_id,
            ParentKind::Comment,
            agoramesh_core::MessageId::from_hex(&top_comment_id).expect("parse top comment id"),
            "Second reply",
            created_at,
        )
        .expect("create second reply");
        let second_reply_id = second_reply.id().to_hex();
        store
            .insert(second_reply, &clock)
            .expect("insert second reply");

        let thread = backend.load_thread(&post_id).expect("load thread");
        assert_eq!(thread.post.text, "Hello from the thread view");
        assert_eq!(thread.comments.len(), 1);

        let comment = thread.comments.first().expect("top-level comment");
        assert_eq!(comment.text, "Top-level comment");
        assert_eq!(comment.replies.len(), 2);

        let expected_reply_ids = {
            let mut ids = vec![first_reply_id, second_reply_id];
            ids.sort();
            ids
        };
        let loaded_reply_ids: Vec<_> = comment
            .replies
            .iter()
            .map(|reply| reply.object_id.as_str())
            .collect();
        assert_eq!(loaded_reply_ids, expected_reply_ids);
    }

    #[test]
    fn backend_generates_dev_plaintext_key_only_when_flagged() {
        let (backend, _temp_dir) = backend_fixture(true);
        let status = backend.key_status(true).expect("key status");
        assert!(
            matches!(status, KeyStatus::Present { .. }),
            "plaintext backend should generate a dev key on demand"
        );
    }
}
