use std::io::{Stdout, stdout};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use color_eyre::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, poll, read};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::compose::submit_compose;
use crate::events::map_event;
use crate::first_seen::compute_warnings;
use crate::key_ux;
use crate::models::{KeyStatus, Screen};
use crate::render::render_shell;

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod terminal_tests;

/// Run the terminal UI until exit.
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

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    backend: &Backend,
    state: &mut AppState,
) -> color_eyre::Result<()> {
    let tick_rate = Duration::from_millis(250);
    while !state.should_quit {
        terminal.draw(|frame| render_shell(state, frame.area(), frame.buffer_mut()))?;
        if poll(tick_rate)? {
            let event = read()?;
            if let Some(action) = map_event(&event, state.screen) {
                if let Some(next_action) = handle_action(backend, state, action)? {
                    if matches!(next_action, Action::Quit) {
                        state.should_quit = true;
                    }
                }
            }
        } else {
            sleep(Duration::from_millis(50));
        }
    }
    Ok(())
}

fn handle_action(
    backend: &Backend,
    state: &mut AppState,
    action: Action,
) -> Result<Option<Action>> {
    match action {
        Action::Quit => {
            *state = state.clone().apply(Action::Quit);
            Ok(Some(Action::Quit))
        }
        Action::Select => handle_select(backend, state),
        Action::ComposeSubmit => handle_compose_submit(backend, state),
        Action::GenerateDevKey => {
            let key_status = key_ux::generate_dev_key(backend)?;
            state.key_status = key_status;
            state.status_message = Some("Development key generated".to_owned());
            Ok(None)
        }
        Action::ToggleSelectedSubscription => {
            let next = state.clone().apply(Action::ToggleSelectedSubscription);
            backend.save_subscriptions(&next.subscriptions)?;
            *state = next;
            state.status_message = Some("Subscriptions updated".to_owned());
            Ok(None)
        }
        other => {
            *state = state.clone().apply(other);
            Ok(None)
        }
    }
}

fn handle_select(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    if let Some(warning) = state.warnings.first().cloned() {
        let next = state.clone().apply(Action::AcknowledgeWarning(warning));
        backend.save_acknowledged(&next.acknowledged)?;
        *state = next;
        return Ok(None);
    }

    if state.screen != Screen::Feed {
        return Ok(None);
    }

    let Some(category) = state.categories.get(state.selected_index) else {
        state.status_message = Some("No posts available for the selected category".to_owned());
        return Ok(None);
    };

    let Some(post) = state
        .posts
        .get(&category.category_id)
        .and_then(|posts| posts.last())
        .cloned()
    else {
        state.status_message = Some("No posts available for the selected category".to_owned());
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

fn handle_compose_submit(backend: &Backend, state: &mut AppState) -> Result<Option<Action>> {
    let compose = state.compose.clone();
    let category_id = state
        .categories
        .get(compose.category_index)
        .map(|category| category.category_id.clone())
        .ok_or_else(|| crate::error::Error::Message("no category selected".to_owned()))?;
    let post = submit_compose(backend, state, &compose).inspect_err(|error| {
        let message = error.to_string();
        state.compose.status = Some(message.clone());
        state.status_message = Some(message);
    })?;

    state.posts.entry(category_id).or_default().push(post);
    state.compose.text.clear();
    state.compose.preview = false;
    state.compose.status = Some("Post submitted".to_owned());
    state.status_message = Some("Post submitted".to_owned());
    state.screen = Screen::Feed;
    state.screen_stack.clear();
    state.selected_index = 0;
    Ok(None)
}
