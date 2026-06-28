//! Peer and sync status rendering for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::app::AppState;
use crate::models::SyncTotals;

/// Renders the sync status screen.
pub fn render_sync_status(state: &AppState, area: Rect, buf: &mut Buffer) {
    if state.peers.is_empty() {
        let empty = Paragraph::new(
            "수동 피어가 설정되지 않았습니다. Phase 2에서는 TUI가 탐색이나 백그라운드 동기화를 실행하지 않습니다.",
        )
                .block(Block::default().borders(Borders::ALL).title("동기화 상태"))
                .wrap(Wrap { trim: true });
        empty.render(area, buf);
        return;
    }

    let mut lines: Vec<Line<'_>> = Vec::new();
    lines.push(Line::from(
        "설정된 수동 피어가 있습니다. Phase 2에서는 백그라운드 동기화를 실행하지 않습니다.",
    ));
    lines.push(Line::from(
        "명시적 동기화 결과가 연결된 뒤에만 마지막 실행 상태를 표시합니다.",
    ));
    lines.push(Line::from(""));
    for peer in &state.peers {
        let status = match peer.last_sync_ok {
            None => "동기화한 적 없음".to_owned(),
            Some(true) => "마지막 동기화 성공".to_owned(),
            Some(false) => "마지막 동기화 실패".to_owned(),
        };
        let name = peer.name.as_deref().unwrap_or("이름 없음");
        lines.push(Line::from(vec![Span::raw(format!(
            "{} ({}) — {}",
            name, peer.address, status
        ))]));
    }
    if state.sync_totals != SyncTotals::default() {
        let totals = format!(
            "마지막 명시적 동기화: 가져옴 {} | 보냄 {} | 거부됨 {}",
            state.sync_totals.pulled, state.sync_totals.pushed, state.sync_totals.rejected
        );
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            totals,
            Style::default().fg(Color::DarkGray),
        )]));
    }
    let block = Block::default().borders(Borders::ALL).title("동기화 상태");
    Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .render(area, buf);
}

/// Returns a compact label for sync totals.
#[must_use]
pub fn totals_label(totals: &SyncTotals) -> String {
    format!(
        "가져옴 {} | 보냄 {} | 거부됨 {}",
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
        let compact = text.replace(' ', "");
        assert!(compact.contains("수동피어가설정되지않았습니다"));
        assert!(compact.contains("백그라운드"));
        assert!(compact.contains("화를실행하지않습니다"));
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
        let compact = text.replace(' ', "");
        assert!(compact.contains("가져옴3"));
        assert!(compact.contains("보냄2"));
        assert!(compact.contains("거부됨1"));
    }
}
