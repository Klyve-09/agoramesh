//! Shared TUI rendering helpers and screen layout.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::AppState;
use crate::compose::ComposeState;
use crate::first_seen::render_warnings as render_first_seen_warnings;
use crate::models::{Screen, SyncTotals};

/// Renders the full application shell into the provided buffer area.
pub fn render_shell(state: &AppState, area: Rect, buf: &mut Buffer) {
    let layout = shell_layout(state, area);
    render_header(state, layout.header, buf);
    render_first_seen_warnings(state, layout.warnings, buf);
    render_body(state, layout.body, buf);
    render_footer(state, layout.footer, buf);
}

/// Renders the body area according to the current screen.
pub fn render_body_for_screen(
    state: &AppState,
    compose: &ComposeState,
    area: Rect,
    buf: &mut Buffer,
) {
    match state.screen {
        Screen::Feed => crate::feed::render_feed(state, area, buf),
        Screen::Compose => crate::compose::render_compose(state, compose, area, buf),
        Screen::Thread => crate::thread::render_thread(state, area, buf),
        Screen::Subscriptions => render_subscriptions(state, area, buf),
        Screen::SyncStatus => crate::sync_status::render_sync_status(state, area, buf),
        Screen::KeyManagement => crate::key_ux::render_key_management(state, area, buf),
    }
}

fn render_subscriptions(state: &AppState, area: Rect, buf: &mut Buffer) {
    let lines: Vec<Line<'_>> = state
        .categories
        .iter()
        .enumerate()
        .map(|(index, category)| {
            let selected = state.screen == Screen::Subscriptions && index == state.selected_index;
            let subscribed = state
                .subscriptions
                .category_ids
                .contains(&category.category_id);
            let marker = if subscribed { "[x]" } else { "[ ]" };
            let label = format!(
                "{marker} {} ({})",
                category.display_name, category.category_id
            );
            let style = if selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Line::from(Span::styled(label, style))
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Subscriptions — Space/Enter toggles, all known categories shown");
    Paragraph::new(lines).block(block).render(area, buf);
}

struct ShellLayout {
    header: Rect,
    warnings: Rect,
    body: Rect,
    footer: Rect,
}

fn shell_layout(state: &AppState, area: Rect) -> ShellLayout {
    let warning_height = if state.warnings.is_empty() || area.height <= 4 {
        0
    } else {
        u16::try_from(state.warnings.len().saturating_add(1))
            .unwrap_or(3)
            .min(4)
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(warning_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    ShellLayout {
        header: *chunks.first().unwrap_or(&area),
        warnings: *chunks.get(1).unwrap_or(&area),
        body: *chunks.get(2).unwrap_or(&area),
        footer: *chunks.get(3).unwrap_or(&area),
    }
}

fn render_header(state: &AppState, area: Rect, buf: &mut Buffer) {
    let title = format!("AgoraMesh — {:?}", state.screen);
    let header = Paragraph::new(title).style(Style::default().bold().fg(Color::Cyan));
    header.render(area, buf);
}

fn render_body(state: &AppState, area: Rect, buf: &mut Buffer) {
    render_body_for_screen(state, &state.compose, area, buf);
}

fn render_footer(state: &AppState, area: Rect, buf: &mut Buffer) {
    let help =
        "1:Feed 2:Subs 3:Sync 4:Key n:New Tab:Focus a:Ack Enter:Open/Submit Esc:Back Ctrl+q:Quit";
    let footer_text = state
        .status_message
        .as_ref()
        .map_or_else(|| help.to_owned(), |message| format!("{help} | {message}"));
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    footer.render(area, buf);
}

/// Renders sync totals in a compact form for status panels.
pub fn render_sync_totals(totals: &SyncTotals, area: Rect, buf: &mut Buffer) {
    let text = format!(
        "pulled {} | pushed {} | rejected {}",
        totals.pulled, totals.pushed, totals.rejected
    );
    Paragraph::new(text).render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::FirstSeenWarning;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn shell_renders_navigation_and_help() {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        let state = AppState::new();
        terminal
            .draw(|frame| render_shell(&state, frame.area(), frame.buffer_mut()))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("AgoraMesh"));
        assert!(text.contains("Feed"));
        assert!(text.contains("Tab:Focus"));
    }

    #[test]
    fn first_seen_render_shell_shows_warnings() {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        let mut state = AppState::new();
        state.warnings.push(FirstSeenWarning::Peer {
            address: "http://127.0.0.1:8080".to_owned(),
        });

        terminal
            .draw(|frame| render_shell(&state, frame.area(), frame.buffer_mut()))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("First time seeing peer"));
    }
}
