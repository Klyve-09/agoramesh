//! Feed, thread, and post creation projections over the verified store.

use agoramesh_core::SystemClock;
use agoramesh_core::objects::acceptance::{self, AcceptanceContext};
use agoramesh_core::objects::projection::{self, CategoryObject, CommentObject, PostObject};
use agoramesh_core::objects::{ParentKind, post as post_obj};
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
        let context = AcceptanceContext::phase1(&clock);
        let mut summaries = Vec::with_capacity(messages.len());
        for message in messages {
            acceptance::validate_phase1_for_acceptance(&message, &context).map_err(Error::from)?;
            let body = projection::decode_body::<CategoryObject>(&message).map_err(Error::from)?;
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
        let context = AcceptanceContext::phase1(&clock);
        let mut posts = Vec::new();
        for message in messages {
            let Some(body) =
                projection::maybe_decode_body::<PostObject>(&message).map_err(Error::from)?
            else {
                continue;
            };
            acceptance::validate_phase1_for_acceptance(&message, &context).map_err(Error::from)?;
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
        acceptance::validate_phase1_for_acceptance(&message, &AcceptanceContext::phase1(&clock))
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
            .map_err(|error| Error::Message(format!("게시글 ID가 올바르지 않습니다: {error}")))?;
        let post_message = store
            .get(post_message_id, &clock)?
            .ok_or_else(|| Error::Message("게시글을 찾을 수 없습니다".to_owned()))?;
        let Some(post_body) =
            projection::maybe_decode_body::<PostObject>(&post_message).map_err(Error::from)?
        else {
            return Err(Error::Message("스레드 루트가 게시글이 아닙니다".to_owned()));
        };
        let context = AcceptanceContext::phase1(&clock);
        acceptance::validate_phase1_for_acceptance(&post_message, &context).map_err(Error::from)?;
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
            let Some(body) =
                projection::maybe_decode_body::<CommentObject>(&message).map_err(Error::from)?
            else {
                continue;
            };
            let parent_id =
                agoramesh_core::MessageId::from_hex(&body.parent_id).map_err(|error| {
                    Error::Message(format!("댓글 parent_id가 올바르지 않습니다: {error}"))
                })?;
            acceptance::validate_phase1_for_acceptance(&message, &context).map_err(Error::from)?;
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
#[path = "content_tests.rs"]
mod tests;
