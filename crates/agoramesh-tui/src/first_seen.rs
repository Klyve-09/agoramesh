//! First-seen category/peer warning logic for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Widget, Wrap};

use crate::app::AppState;
use crate::models::{AcknowledgedFirstSeen, CategorySummary, FirstSeenWarning, PeerStatus};

/// Computes active first-seen warnings from categories and peers.
#[must_use]
pub fn compute_warnings(
    categories: &[CategorySummary],
    peers: &[PeerStatus],
    acknowledged: &AcknowledgedFirstSeen,
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

pub(crate) fn render_warnings(state: &AppState, area: Rect, buf: &mut Buffer) {
    if state.warnings.is_empty() {
        Clear.render(area, buf);
        return;
    }
    let text = warning_lines(&state.warnings);
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: true });
    paragraph.render(area, buf);
}

fn warning_lines(warnings: &[FirstSeenWarning]) -> Vec<Line<'_>> {
    warnings
        .iter()
        .map(|warning| match warning {
            FirstSeenWarning::Category {
                category_id,
                display_name,
            } => {
                let name = display_name.as_deref().unwrap_or(category_id.as_str());
                Line::from(vec![
                    Span::styled("! ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(
                        "처음 보는 카테고리 '{name}' ({category_id})입니다. 확인하려면 'a'를 누르세요.",
                    )),
                ])
            }
            FirstSeenWarning::Peer { address } => Line::from(vec![
                Span::styled("! ", Style::default().fg(Color::Yellow)),
                Span::raw(format!(
                    "처음 보는 피어 {address}입니다. 확인하려면 'a'를 누르세요."
                )),
            ]),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn sample_category(category_id: &str) -> CategorySummary {
        CategorySummary {
            object_id: "oid".to_owned(),
            display_name: "General".to_owned(),
            description: String::new(),
            category_id: category_id.to_owned(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn first_seen_category_warning_appears_once_after_ack() {
        let category = sample_category("cat-a");
        let acknowledged = AcknowledgedFirstSeen::default();
        let warnings = compute_warnings(std::slice::from_ref(&category), &[], &acknowledged);
        assert_eq!(warnings.len(), 1);

        let mut acknowledged = AcknowledgedFirstSeen::default();
        acknowledged.categories.push("cat-a".to_owned());
        let warnings = compute_warnings(&[category], &[], &acknowledged);
        assert!(warnings.is_empty());
    }

    #[test]
    fn first_seen_peer_warning_appears_once_after_ack() {
        let peer = PeerStatus {
            name: None,
            address: "http://127.0.0.1:8080".to_owned(),
            last_sync_ok: None,
        };
        let acknowledged = AcknowledgedFirstSeen::default();
        let warnings = compute_warnings(&[], std::slice::from_ref(&peer), &acknowledged);
        assert_eq!(warnings.len(), 1);

        let mut acknowledged = AcknowledgedFirstSeen::default();
        acknowledged.peers.push("http://127.0.0.1:8080".to_owned());
        let warnings = compute_warnings(&[], &[peer], &acknowledged);
        assert!(warnings.is_empty());
    }

    #[test]
    fn first_seen_render_warnings_shows_warning_text() {
        let mut terminal = Terminal::new(TestBackend::new(80, 5)).expect("terminal");
        let mut state = AppState::new();
        state.warnings.push(FirstSeenWarning::Peer {
            address: "http://127.0.0.1:8080".to_owned(),
        });

        terminal
            .draw(|frame| render_warnings(&state, frame.area(), frame.buffer_mut()))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        let compact = text.replace(' ', "");
        assert!(compact.contains("처음보는피어"));
        assert!(compact.contains("'a'를누르세요"));
    }
}
