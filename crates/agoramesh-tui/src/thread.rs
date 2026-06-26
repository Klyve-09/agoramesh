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
        render_comments(&thread.comments, 0, layout.comments, buf);
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

fn render_comments(comments: &[ThreadComment], depth: usize, area: Rect, buf: &mut Buffer) {
    let indent = "  ".repeat(depth);
    let mut lines: Vec<Line<'_>> = Vec::new();
    for comment in comments {
        let marker = if comment.collapsed { "+ " } else { "- " };
        let header = format!(
            "{indent}{marker}{} | {}",
            short_id(&comment.object_id),
            short_id(&comment.author_id)
        );
        lines.push(Line::from(vec![Span::styled(
            header,
            Style::default().fg(Color::DarkGray),
        )]));
        if !comment.collapsed {
            for body_line in comment.text.lines() {
                lines.push(Line::from(format!("{indent}  {body_line}")));
            }
            let mut reply_lines = Vec::new();
            let mut reply_buffer = Buffer::empty(area);
            render_comments(
                &comment.replies,
                depth.saturating_add(1),
                area,
                &mut reply_buffer,
            );
            for line in reply_buffer.content.chunks(area.width as usize) {
                let line_text: String = line.iter().map(ratatui::buffer::Cell::symbol).collect();
                reply_lines.push(Line::from(line_text));
            }
            lines.extend(reply_lines);
        }
    }
    let block = Block::default().borders(Borders::ALL).title("Comments");
    Paragraph::new(lines).block(block).render(area, buf);
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
