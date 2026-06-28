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
use crate::controller::{handle_action, load_posts};
use crate::events::map_event;
use crate::first_seen::compute_warnings;
use crate::models::KeyStatus;
use crate::render::render_shell;

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod terminal_tests;

/// Run the terminal UI until exit.
pub fn run(data_dir: Option<PathBuf>, plaintext: bool) -> Result<()> {
    let backend = Backend::open(data_dir, plaintext)?;
    let mut state = initialize_state(&backend)?;
    let mut terminal = TerminalGuard::enter()?;
    run_event_loop(terminal.terminal_mut(), &backend, &mut state)
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

#[derive(Default)]
struct TerminalSetupProgress {
    raw_mode: bool,
    alternate_screen: bool,
    mouse_capture: bool,
}

#[derive(Default)]
struct TerminalSetupGuard {
    progress: TerminalSetupProgress,
    active: bool,
}

trait TerminalSetupCleanup {
    fn disable_mouse_capture(&mut self) -> std::io::Result<()>;
    fn leave_alternate_screen(&mut self) -> std::io::Result<()>;
    fn disable_raw_mode(&mut self) -> std::io::Result<()>;
}

struct StdoutSetupCleanup;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        let terminal = setup_terminal()?;
        Ok(Self { terminal })
    }

    const fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal(&mut self.terminal);
    }
}

impl TerminalSetupGuard {
    fn new() -> Self {
        Self {
            progress: TerminalSetupProgress::default(),
            active: true,
        }
    }

    const fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for TerminalSetupGuard {
    fn drop(&mut self) {
        if self.active {
            let mut cleanup = StdoutSetupCleanup;
            let _ = cleanup_terminal_setup(&self.progress, &mut cleanup);
        }
    }
}

impl TerminalSetupCleanup for StdoutSetupCleanup {
    fn disable_mouse_capture(&mut self) -> std::io::Result<()> {
        stdout().execute(DisableMouseCapture).map(|_stdout| ())
    }

    fn leave_alternate_screen(&mut self) -> std::io::Result<()> {
        stdout().execute(LeaveAlternateScreen).map(|_stdout| ())
    }

    fn disable_raw_mode(&mut self) -> std::io::Result<()> {
        disable_raw_mode()
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut setup_guard = TerminalSetupGuard::new();
    enable_raw_mode()?;
    setup_guard.progress.raw_mode = true;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    setup_guard.progress.alternate_screen = true;
    stdout.execute(EnableMouseCapture)?;
    setup_guard.progress.mouse_capture = true;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    setup_guard.disarm();
    Ok(terminal)
}

fn cleanup_terminal_setup(
    progress: &TerminalSetupProgress,
    cleanup: &mut impl TerminalSetupCleanup,
) -> std::io::Result<()> {
    let mut first_error = None;
    if progress.mouse_capture {
        record_cleanup_result(cleanup.disable_mouse_capture(), &mut first_error);
    }
    if progress.alternate_screen {
        record_cleanup_result(cleanup.leave_alternate_screen(), &mut first_error);
    }
    if progress.raw_mode {
        record_cleanup_result(cleanup.disable_raw_mode(), &mut first_error);
    }
    first_error.map_or(Ok(()), Err)
}

fn record_cleanup_result(result: std::io::Result<()>, first_error: &mut Option<std::io::Error>) {
    if let Err(error) = result {
        if first_error.is_none() {
            *first_error = Some(error);
        }
    }
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    let raw_result = disable_raw_mode();
    let leave_result = terminal
        .backend_mut()
        .execute(LeaveAlternateScreen)
        .map(|_stdout| ());
    let mouse_result = terminal
        .backend_mut()
        .execute(DisableMouseCapture)
        .map(|_stdout| ());
    raw_result?;
    leave_result?;
    mouse_result?;
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
    state.posts = load_posts(backend, &state.categories, &state)?;
    if matches!(state.key_status, KeyStatus::Missing) {
        state.status_message =
            Some("신원 키를 찾을 수 없습니다. 키 관리(4)를 열어 생성하세요.".to_owned());
    }
    Ok(state)
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
