//! Terminal lifecycle and main event loop for the TUI.

use std::io::{Stdout, stdout};
use std::path::PathBuf;
use std::time::Duration;

use color_eyre::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, poll, read};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::thread::sleep;

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::events::map_event;
use crate::models::{FirstSeenWarning, KeyStatus};
use crate::render::render_shell;

/// Runs the TUI until the user quits.
///
/// The current implementation initializes the terminal, opens the backend, loads
/// initial state, and runs a minimal event loop. It is intentionally simple for
/// Phase 2 and will be extended with real backend polling in later phases.
pub fn run(data_dir: Option<PathBuf>, plaintext: bool, _allow_public_bind: bool) -> Result<()> {
    let backend = Backend::open(data_dir, plaintext)?;
    let mut terminal = setup_terminal()?;
    let mut state = initialize_state(&backend)?;
    let result = run_event_loop(&mut terminal, &backend, &mut state);
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)?
        .execute(DisableMouseCapture)?;
    Ok(())
}

fn initialize_state(backend: &Backend) -> color_eyre::Result<AppState> {
    let mut state = AppState::new();
    state.subscriptions = backend.load_subscriptions()?;
    state.acknowledged = backend.load_acknowledged()?;
    state.key_status = backend.key_status(true)?;
    state.categories = backend.load_categories()?;
    state.peers = backend.peer_statuses()?;
    state.warnings = compute_warnings(&state.categories, &state.peers, &state.acknowledged);
    state.posts = load_posts(backend, &state.categories)?;
    if matches!(state.key_status, KeyStatus::Missing) {
        state.status_message =
            Some("No identity key found. Open Key Management (4) to generate one.".to_owned());
    }
    Ok(state)
}

fn load_posts(
    backend: &Backend,
    categories: &[crate::models::CategorySummary],
) -> color_eyre::Result<std::collections::HashMap<String, Vec<crate::models::FeedPost>>> {
    let mut posts = std::collections::HashMap::new();
    for category in categories {
        let category_posts = backend.load_feed(&category.category_id)?;
        posts.insert(category.category_id.clone(), category_posts);
    }
    Ok(posts)
}

fn compute_warnings(
    categories: &[crate::models::CategorySummary],
    peers: &[crate::models::PeerStatus],
    acknowledged: &crate::models::AcknowledgedFirstSeen,
) -> Vec<FirstSeenWarning> {
    let mut warnings = Vec::new();
    for category in categories {
        if !acknowledged.categories.contains(&category.category_id) {
            warnings.push(FirstSeenWarning::Category {
                category_id: category.category_id.clone(),
                display_name: Some(category.display_name.clone()),
            });
        }
    }
    for peer in peers {
        if !acknowledged.peers.contains(&peer.address) {
            warnings.push(FirstSeenWarning::Peer {
                address: peer.address.clone(),
            });
        }
    }
    warnings
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    _backend: &Backend,
    state: &mut AppState,
) -> color_eyre::Result<()> {
    let tick_rate = Duration::from_millis(250);
    while !state.should_quit {
        terminal.draw(|frame| render_shell(state, frame.area(), frame.buffer_mut()))?;
        if poll(tick_rate)? {
            let event = read()?;
            if let Some(action) = map_event(&event) {
                if action == Action::Quit {
                    state.should_quit = true;
                } else {
                    let updated = state.clone().apply(action);
                    *state = updated;
                }
            }
        } else {
            sleep(Duration::from_millis(50));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_warnings_lists_unacknowledged_categories_and_peers() {
        let category = crate::models::CategorySummary {
            object_id: "oid".to_owned(),
            display_name: "General".to_owned(),
            description: "General chat".to_owned(),
            category_id: "cat-general".to_owned(),
            created_at: chrono::Utc::now(),
        };
        let peer = crate::models::PeerStatus {
            name: None,
            address: "http://127.0.0.1:8080".to_owned(),
            last_sync_ok: None,
        };
        let acknowledged = crate::models::AcknowledgedFirstSeen::default();
        let warnings = compute_warnings(&[category], &[peer], &acknowledged);
        assert_eq!(warnings.len(), 2);
    }
}
