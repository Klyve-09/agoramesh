//! Navigation action handlers for the TUI controller.

use color_eyre::Result;

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::models::Screen;

/// Handles the primary Select action based on the current screen.
pub(super) fn handle_select(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
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
