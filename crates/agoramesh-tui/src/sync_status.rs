//! Peer and sync status rendering for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::AppState;
use crate::models::SyncTotals;

/// Renders the sync status screen.
pub fn render_sync_status(state: &AppState, area: Rect, buf: &mut Buffer) {
    if state.peers.is_empty() {
        let empty =
            Paragraph::new("No peers configured. Add peers with the CLI or a future TUI flow.")
                .block(Block::default().borders(Borders::ALL).title("Sync Status"));
        empty.render(area, buf);
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();
    for peer in &state.peers {
        let status = match peer.last_sync_ok {
            None => "never synced".to_owned(),
            Some(true) => "last sync ok".to_owned(),
            Some(false) => "last sync failed".to_owned(),
        };
        let name = peer.name.as_deref().unwrap_or("unnamed");
        lines.push(Line::from(vec![Span::raw(format!(
            "{} ({}) — {}",
            name, peer.address, status
        ))]));
    }
    let totals = format!(
        "Totals: pulled {} | pushed {} | rejected {}",
        state.sync_totals.pulled, state.sync_totals.pushed, state.sync_totals.rejected
    );
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        totals,
        Style::default().fg(Color::DarkGray),
    )]));
    let block = Block::default().borders(Borders::ALL).title("Sync Status");
    Paragraph::new(lines).block(block).render(area, buf);
}

/// Returns a compact label for sync totals.
#[must_use]
pub fn totals_label(totals: &SyncTotals) -> String {
    format!(
        "pulled {} | pushed {} | rejected {}",
        totals.pulled, totals.pushed, totals.rejected
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PeerStatus, SyncTotals};

    #[test]
    fn sync_status_shows_no_peers_when_empty() {
        let state = crate::app::AppState::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        render_sync_status(&state, buffer.area, &mut buffer);
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("No peers configured"));
    }

    #[test]
    fn sync_status_reports_last_sync_totals() {
        let mut state = crate::app::AppState::new();
        state.peers = vec![PeerStatus {
            name: Some("local".to_owned()),
            address: "http://127.0.0.1:8080".to_owned(),
            last_sync_ok: Some(true),
        }];
        state.sync_totals = SyncTotals {
            pulled: 3,
            pushed: 2,
            rejected: 1,
        };
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        render_sync_status(&state, buffer.area, &mut buffer);
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("pulled 3"));
        assert!(text.contains("pushed 2"));
        assert!(text.contains("rejected 1"));
    }
}
