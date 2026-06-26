//! Backend-backed TUI action controller.

use std::collections::HashMap;

use color_eyre::Result;

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::compose::submit_compose;
use crate::first_seen::compute_warnings;
use crate::key_ux;
use crate::models::{CategorySummary, FeedFocus, FeedPost, Screen};

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
        Action::Select => handle_select(backend, state),
        Action::AcknowledgeCurrentWarning => acknowledge_current_warning(backend, state),
        Action::ComposeSubmit => Ok(handle_compose_submit(backend, state)),
        Action::GenerateDevKey => handle_generate_dev_key(backend, state),
        Action::GenerateEncryptedKey => handle_generate_encrypted_key(backend, state),
        Action::UnlockKey => Ok(handle_unlock_key(backend, state)),
        Action::BackupKey => handle_backup_key(backend, state),
        Action::RestoreKey => handle_restore_key(backend, state),
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

fn handle_select(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    match state.screen {
        Screen::Feed => open_selected_thread(backend, state),
        Screen::Thread => {
            *state = state.clone().apply(Action::ToggleCollapse);
            Ok(None)
        }
        Screen::Subscriptions | Screen::SyncStatus | Screen::KeyManagement | Screen::Compose => {
            Ok(None)
        }
    }
}

fn open_selected_thread(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    let Some(post) = state.selected_feed_post().cloned() else {
        state.status_message = Some("No post selected in the current feed category".to_owned());
        return Ok(None);
    };
    let thread = backend.load_thread(&post.object_id)?;
    let next = state
        .clone()
        .apply(Action::SetThread(thread))
        .apply(Action::SetScreen(Screen::Thread));
    *state = next;
    Ok(None)
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

fn handle_compose_submit(backend: &Backend, state: &mut AppState) -> Option<Action> {
    if !state.compose.preview {
        state.compose.text.push('\n');
        return None;
    }

    if state.compose.text.trim().is_empty() {
        let message = "post text is empty".to_owned();
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
        state.status_message = Some("no category selected".to_owned());
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
            state.compose.status = Some("Post submitted".to_owned());
            state.status_message = Some("Post submitted".to_owned());
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

fn handle_generate_dev_key(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    let key_status = key_ux::generate_dev_key(backend)?;
    state.key_status = key_status;
    state.key_input.status = Some("Development key generated".to_owned());
    state.status_message = Some("Development key generated".to_owned());
    Ok(None)
}

fn handle_generate_encrypted_key(
    backend: &Backend,
    state: &mut AppState,
) -> Result<Option<Action>> {
    if state.key_input.passphrase.is_empty() {
        let message = "type a passphrase before generating an encrypted key".to_owned();
        state.key_input.status = Some(message.clone());
        state.status_message = Some(message);
        return Ok(None);
    }
    let status = backend.generate_encrypted_key(&state.key_input.passphrase)?;
    state.key_status = status;
    state.key_input.passphrase.clear();
    state.key_input.status = Some("Encrypted key generated and unlocked".to_owned());
    state.status_message = Some("Encrypted key generated and unlocked".to_owned());
    Ok(None)
}

fn handle_unlock_key(backend: &Backend, state: &mut AppState) -> Option<Action> {
    if state.key_input.passphrase.is_empty() {
        let message = "type a passphrase before unlocking the key".to_owned();
        state.key_input.status = Some(message.clone());
        state.status_message = Some(message);
        return None;
    }
    match backend.unlock_key(&state.key_input.passphrase) {
        Ok(status) => {
            state.key_status = status;
            state.key_input.passphrase.clear();
            state.key_input.status = Some("Encrypted key unlocked".to_owned());
            state.status_message = Some("Encrypted key unlocked".to_owned());
        }
        Err(error) => {
            let message = error.to_string();
            state.key_input.status = Some(message.clone());
            state.status_message = Some(message);
        }
    }
    None
}

fn handle_backup_key(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    let path = backend.backup_key()?;
    let message = format!("Key backup written to {}", path.display());
    state.key_input.status = Some(message.clone());
    state.status_message = Some(message);
    Ok(None)
}

fn handle_restore_key(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    backend.restore_key_from_backup()?;
    state.key_status = backend.key_status(false)?;
    state.key_input.status = Some("Key restored from backup".to_owned());
    state.status_message = Some("Key restored from backup".to_owned());
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

fn visible_category_index(state: &AppState, category_id: &str) -> Option<usize> {
    state
        .categories
        .iter()
        .filter(|category| state.subscriptions.category_ids.contains(&category.category_id))
        .position(|category| category.category_id == category_id)
}
