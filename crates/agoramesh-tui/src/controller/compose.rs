//! Compose submission action for the TUI controller.

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::compose::submit_compose;
use crate::models::{FeedFocus, Screen};

/// Handles submitting the composed post, updating feed selection on success.
pub(super) fn handle_compose_submit(backend: &Backend, state: &mut AppState) -> Option<Action> {
    if !state.compose.preview {
        state.compose.text.push('\n');
        return None;
    }

    if state.compose.text.trim().is_empty() {
        let message = "게시글 내용이 비어 있습니다".to_owned();
        state.compose.status = Some(message.clone());
        state.status_message = Some(message);
        return None;
    }

    let compose = state.compose.clone();
    let Some(category_id) = state
        .categories
        .get(compose.category_index)
        .map(|category| category.category_id.clone())
    else {
        state.status_message = Some("선택한 카테고리가 없습니다".to_owned());
        return None;
    };
    match submit_compose(backend, state, &compose) {
        Ok(post) => {
            let selected_category_index = visible_category_index(state, &category_id);
            let posts = state.posts.entry(category_id).or_default();
            posts.push(post);
            let selected_post_index = posts.len().saturating_sub(1);
            state.compose.text.clear();
            state.compose.preview = false;
            state.compose.status = Some("게시글을 제출했습니다".to_owned());
            state.status_message = Some("게시글을 제출했습니다".to_owned());
            state.screen = Screen::Feed;
            state.screen_stack.clear();
            if let Some(index) = selected_category_index {
                state.selected_category_index = index;
                state.selected_post_index = selected_post_index;
                state.feed_focus = FeedFocus::Posts;
            } else {
                state.clamp_feed_post_index();
            }
            None
        }
        Err(error) => {
            let message = error.to_string();
            state.compose.status = Some(message.clone());
            state.status_message = Some(message);
            None
        }
    }
}

fn visible_category_index(state: &AppState, category_id: &str) -> Option<usize> {
    state
        .categories
        .iter()
        .filter(|category| {
            state
                .subscriptions
                .category_ids
                .contains(&category.category_id)
        })
        .position(|category| category.category_id == category_id)
}
