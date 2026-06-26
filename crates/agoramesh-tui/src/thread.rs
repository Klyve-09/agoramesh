//! Thread view model and rendering for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::AppState;
use crate::models::{FeedPost, ThreadComment};

/// Renders the thread screen for a selected post.
pub fn render_thread(state: &AppState, area: Rect, buf: &mut Buffer) {
    let layout = thread_layout(area);
    if let Some(thread) = &state.thread {
        render_post(&thread.post, layout.post, buf);
        render_comments(&thread.comments, state.selected_index, layout.comments, buf);
    } else {
        let empty = Paragraph::new("No post selected. Return to feed and press Enter.")
            .block(Block::default().borders(Borders::ALL).title("Thread"));
        empty.render(area, buf);
    }
}

struct ThreadLayout {
    post: Rect,
    comments: Rect,
}

fn thread_layout(area: Rect) -> ThreadLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(area);
    ThreadLayout {
        post: *chunks.first().unwrap_or(&area),
        comments: *chunks.get(1).unwrap_or(&area),
    }
}

fn render_post(post: &FeedPost, area: Rect, buf: &mut Buffer) {
    let text = format!(
        "{short_id} | {author}\n{text}",
        short_id = short_id(&post.object_id),
        author = short_id(&post.author_id),
        text = post.text
    );
    Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Post"))
        .render(area, buf);
}

fn render_comments(
    comments: &[ThreadComment],
    selected_index: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    flatten_comments(comments, 0, selected_index, &mut 0, &mut lines);
    let block = Block::default().borders(Borders::ALL).title("Comments");
    Paragraph::new(lines).block(block).render(area, buf);
}

fn flatten_comments(
    comments: &[ThreadComment],
    depth: usize,
    selected_index: usize,
    row_index: &mut usize,
    lines: &mut Vec<Line<'static>>,
) {
    let indent = "  ".repeat(depth);
    for comment in comments {
        let selected = *row_index == selected_index;
        let marker = if comment.collapsed { "+ " } else { "- " };
        let cursor = if selected { "> " } else { "  " };
        let header = format!(
            "{cursor}{indent}{marker}{} | {}",
            short_id(&comment.object_id),
            short_id(&comment.author_id)
        );
        let style = if selected {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(vec![Span::styled(header, style)]));
        *row_index = row_index.saturating_add(1);
        if !comment.collapsed {
            for body_line in comment.text.lines() {
                lines.push(Line::from(format!("  {indent}{body_line}")));
            }
            flatten_comments(
                &comment.replies,
                depth.saturating_add(1),
                selected_index,
                row_index,
                lines,
            );
        }
    }
}

fn short_id(id: &str) -> String {
    if id.len() > 12 {
        format!("{}...", id.get(..12).unwrap_or(id))
    } else {
        id.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ThreadView;
    use chrono::Utc;

    fn sample_thread() -> ThreadView {
        ThreadView {
            post: FeedPost {
                object_id: "post-id".to_owned(),
                author_id: "author".to_owned(),
                text: "Root post".to_owned(),
                created_at: Utc::now(),
            },
            comments: vec![ThreadComment {
                object_id: "comment-1".to_owned(),
                author_id: "author".to_owned(),
                text: "First comment".to_owned(),
                created_at: Utc::now(),
                replies: vec![ThreadComment {
                    object_id: "reply-1".to_owned(),
                    author_id: "author".to_owned(),
                    text: "Nested reply".to_owned(),
                    created_at: Utc::now(),
                    replies: vec![],
                    collapsed: false,
                }],
                collapsed: false,
            }],
        }
    }

    #[test]
    fn thread_builds_comment_tree_under_post() {
        let mut state = AppState::new();
        state.thread = Some(sample_thread());
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        render_thread(&state, buffer.area, &mut buffer);
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("Root post"));
        assert!(text.contains("First comment"));
        assert!(text.contains("Nested reply"));
    }

    #[test]
    fn thread_collapse_hides_descendants_without_losing_state() {
        let mut state = AppState::new();
        state.thread = Some(sample_thread());
        state.selected_index = 0;
        state = state.apply(crate::app::Action::ToggleCollapse);
        assert!(
            state
                .thread
                .as_ref()
                .and_then(|thread| thread.comments.first())
                .is_some_and(|comment| comment.collapsed)
        );
    }
}
