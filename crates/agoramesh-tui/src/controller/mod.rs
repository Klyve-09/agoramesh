//! Backend-backed TUI action controller.

use std::collections::HashMap;

use color_eyre::Result;

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::first_seen::compute_warnings;
use crate::models::{CategorySummary, FeedPost};

mod compose;
mod key_mgmt;
mod navigation;

/// Handles one UI action, including backend side effects when required.
pub fn handle_action(
    backend: &Backend,
    state: &mut AppState,
    action: Action,
) -> Result<Option<Action>> {
    match action {
        Action::Quit => {
            *state = state.clone().apply(Action::Quit);
            Ok(Some(Action::Quit))
        }
        Action::Noop => Ok(None),
        Action::Select => navigation::handle_select(backend, state),
        Action::AcknowledgeCurrentWarning => acknowledge_current_warning(backend, state),
        Action::ComposeSubmit => Ok(compose::handle_compose_submit(backend, state)),
        Action::GenerateDevKey => Ok(key_mgmt::handle_generate_dev_key(backend, state)),
        Action::GenerateEncryptedKey => Ok(key_mgmt::handle_generate_encrypted_key(backend, state)),
        Action::UnlockKey => Ok(key_mgmt::handle_unlock_key(backend, state)),
        Action::BackupKey => Ok(key_mgmt::handle_backup_key(backend, state)),
        Action::RestoreKey => Ok(key_mgmt::handle_restore_key(backend, state)),
        Action::ToggleSelectedSubscription => handle_toggle_subscription(backend, state),
        other => {
            *state = state.clone().apply(other);
            Ok(None)
        }
    }
}

/// Loads feed posts for the category set shown by the feed.
pub fn load_posts(
    backend: &Backend,
    categories: &[CategorySummary],
    state: &AppState,
) -> Result<HashMap<String, Vec<FeedPost>>> {
    let mut posts = HashMap::new();
    for category in categories {
        if state
            .subscriptions
            .category_ids
            .contains(&category.category_id)
        {
            let category_posts = backend.load_feed(&category.category_id)?;
            posts.insert(category.category_id.clone(), category_posts);
        }
    }
    Ok(posts)
}

fn acknowledge_current_warning(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    let Some(warning) = state.warnings.first().cloned() else {
        return Ok(None);
    };
    let mut next = state.clone().apply(Action::AcknowledgeWarning(warning));
    next.warnings = compute_warnings(&next.categories, &next.peers, &next.acknowledged);
    backend.save_acknowledged(&next.acknowledged)?;
    *state = next;
    state.status_message = Some("Warning acknowledged".to_owned());
    Ok(None)
}

fn handle_toggle_subscription(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    let previous = state.subscriptions.category_ids.clone();
    let mut next = state.clone().apply(Action::ToggleSelectedSubscription);
    for category_id in &next.subscriptions.category_ids {
        if !previous.contains(category_id) {
            next.posts
                .insert(category_id.clone(), backend.load_feed(category_id)?);
        }
    }
    for category_id in &previous {
        if !next.subscriptions.category_ids.contains(category_id) {
            next.posts.remove(category_id);
        }
    }
    backend.save_subscriptions(&next.subscriptions)?;
    *state = next;
    state.clamp_feed_post_index();
    state.status_message = Some("Subscriptions updated".to_owned());
    Ok(None)
}
