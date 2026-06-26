//! Feed, thread, and post creation projections over the verified store.

use agoramesh_core::SystemClock;
use agoramesh_core::objects::{
    ParentKind, category as category_obj, comment as comment_obj, post as post_obj,
};
use agoramesh_store::store::Store;
use chrono::{DateTime, Utc};

use crate::error::Error;
use crate::models::{CategorySummary, FeedPost, ThreadComment, ThreadView};

use super::Backend;

impl Backend {
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
        let keypair = super::key_mgmt::load_keypair(self)?;
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

    /// Loads a post and its nested comment thread.
    ///
    /// # Errors
    ///
    /// Returns an error when the store cannot be read, the post is missing, or a
    /// message fails verification.
    pub fn load_thread(&self, post_id: &str) -> Result<ThreadView, Error> {
        let store = self.store()?;
        let clock = SystemClock;
        let post_message_id = agoramesh_core::MessageId::from_hex(post_id)
            .map_err(|error| Error::Message(format!("invalid post id: {error}")))?;
        let post_message = store
            .get(post_message_id, &clock)?
            .ok_or_else(|| Error::Message("post not found".to_owned()))?;
        let post_body: post_obj::Body =
            serde_json::from_slice(post_message.signed_payload().body())
                .map_err(|error| Error::Message(error.to_string()))?;
        let post = FeedPost {
            object_id: post_message.id().to_hex(),
            author_id: hex::encode(post_message.signed_payload().author_pubkey()),
            text: post_body.text,
            created_at: post_body.created_at,
        };

        let category_id = post_message.signed_payload().scope().to_owned();
        let mut post_children: std::collections::HashMap<
            agoramesh_core::MessageId,
            Vec<LoadedComment>,
        > = std::collections::HashMap::new();
        let mut comment_children: std::collections::HashMap<
            agoramesh_core::MessageId,
            Vec<LoadedComment>,
        > = std::collections::HashMap::new();
        for message in store.list_by_scope(&category_id, &clock)? {
            if message.signed_payload().kind() != "comment" {
                continue;
            }
            let body: comment_obj::Body = serde_json::from_slice(message.signed_payload().body())
                .map_err(|error| Error::Message(error.to_string()))?;
            let parent_id = agoramesh_core::MessageId::from_hex(&body.parent_id)
                .map_err(|error| Error::Message(format!("invalid comment parent_id: {error}")))?;
            let parent_kind = body.parent_kind;
            let loaded = LoadedComment {
                object_id: message.id(),
                object_id_hex: message.id().to_hex(),
                author_id: hex::encode(message.signed_payload().author_pubkey()),
                text: body.text,
                created_at: body.created_at,
            };
            match parent_kind {
                ParentKind::Post => post_children.entry(parent_id).or_default().push(loaded),
                ParentKind::Comment => {
                    comment_children.entry(parent_id).or_default().push(loaded);
                }
            }
        }

        let mut top_level = post_children.remove(&post_message_id).unwrap_or_default();
        sort_comments(&mut top_level);
        let comments = build_comment_tree(
            top_level,
            &mut comment_children,
            &mut std::collections::HashSet::new(),
        );
        Ok(ThreadView { post, comments })
    }
}

#[derive(Debug)]
struct LoadedComment {
    object_id: agoramesh_core::MessageId,
    object_id_hex: String,
    author_id: String,
    text: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl LoadedComment {
    fn into_thread_comment(self, replies: Vec<ThreadComment>) -> ThreadComment {
        ThreadComment {
            object_id: self.object_id_hex,
            author_id: self.author_id,
            text: self.text,
            created_at: self.created_at,
            replies,
            collapsed: false,
        }
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
    comments: Vec<LoadedComment>,
    comment_children: &mut std::collections::HashMap<agoramesh_core::MessageId, Vec<LoadedComment>>,
    visited: &mut std::collections::HashSet<agoramesh_core::MessageId>,
) -> Vec<ThreadComment> {
    comments
        .into_iter()
        .filter_map(|comment| {
            if !visited.insert(comment.object_id) {
                return None;
            }
            let mut replies = comment_children
                .remove(&comment.object_id)
                .unwrap_or_default();
            sort_comments(&mut replies);
            let child_replies = build_comment_tree(replies, comment_children, visited);
            Some(comment.into_thread_comment(child_replies))
        })
        .collect()
}

#[cfg(test)]
#[allow(dead_code, reason = "legacy recursive helper retained for future use")]
fn attach_replies(
    mut comments: Vec<ThreadComment>,
    replies_by_parent: &mut std::collections::HashMap<String, Vec<ThreadComment>>,
) -> Vec<ThreadComment> {
    for comment in &mut comments {
        let mut child_replies = replies_by_parent
            .remove(&comment.object_id)
            .unwrap_or_default();
        child_replies.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.object_id.cmp(&b.object_id))
        });
        comment.replies = attach_replies(child_replies, replies_by_parent);
    }
    comments
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
    fn thread_ignores_comment_with_post_id_but_comment_parent_kind() {
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

        let mismatched = comment_obj::create(
            &keypair,
            &category_id,
            ParentKind::Comment,
            agoramesh_core::MessageId::from_hex(&post_id).expect("parse post id"),
            "Mismatched parent kind",
            created_at,
        )
        .expect("create mismatched comment");
        store.insert(mismatched, &clock).expect("insert mismatched");

        let thread = backend.load_thread(&post_id).expect("load thread");
        assert!(
            thread.comments.is_empty(),
            "a comment with parent_kind=Comment and parent_id=post_id should not appear as a top-level post comment"
        );
    }

    #[test]
    fn thread_rejects_malformed_comment_parent_id() {
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

        // Build a syntactically valid comment body whose parent_id is not a valid MessageId.
        let message = {
            let body = comment_obj::Body {
                category_id: category_id.clone(),
                parent_kind: ParentKind::Post,
                parent_id: "not-a-valid-id".to_owned(),
                text: "bad parent id".to_owned(),
                created_at,
            };
            let canonical = agoramesh_core::canonical::to_vec(&body).expect("canonicalize");
            agoramesh_core::message::Message::create(
                &keypair,
                "comment",
                created_at,
                category_id,
                canonical,
            )
            .expect("create bad comment")
        };
        store.insert(message, &clock).expect("insert bad comment");

        let result = backend.load_thread(&post_id);
        let error =
            result.expect_err("load_thread should fail when a comment has an invalid parent_id");
        assert!(
            error.to_string().contains("invalid comment parent_id"),
            "expected malformed parent_id error, got {error}"
        );
    }
}
