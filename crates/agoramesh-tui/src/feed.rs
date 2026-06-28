//! Feed view model and rendering for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap,
};

use crate::app::AppState;
use crate::models::{FeedFocus, FeedPost, Screen};

/// Renders the feed screen: subscribed categories and posts for the selected one.
pub fn render_feed(state: &AppState, area: Rect, buf: &mut Buffer) {
    let layout = feed_layout(area);
    render_category_list(state, layout.categories, buf);
    render_post_list(state, layout.posts, buf);
}

struct FeedLayout {
    categories: Rect,
    posts: Rect,
}

fn feed_layout(area: Rect) -> FeedLayout {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);
    FeedLayout {
        categories: *chunks.first().unwrap_or(&area),
        posts: *chunks.get(1).unwrap_or(&area),
    }
}

fn render_category_list(state: &AppState, area: Rect, buf: &mut Buffer) {
    let categories = state.visible_feed_categories();
    let items: Vec<ListItem<'_>> = categories
        .iter()
        .enumerate()
        .map(|(index, category)| {
            let marker = if state.feed_focus == FeedFocus::Categories
                && Some(index) == selected_index(state)
            {
                "> "
            } else {
                "* "
            };
            let label = format!("{marker}{}", category.display_name);
            let style = if Some(index) == selected_index(state) {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(label)).style(style)
        })
        .collect();
    let mut list_state = ListState::default();
    list_state.select(selected_index(state));
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("카테고리"));
    StatefulWidget::render(list, area, buf, &mut list_state);
}

fn render_post_list(state: &AppState, area: Rect, buf: &mut Buffer) {
    let posts: Vec<FeedPost> = state.selected_feed_posts().to_vec();

    if posts.is_empty() {
        let empty = Paragraph::new(
            "이 카테고리에 구독한 게시글이 없습니다. 'n'으로 글을 쓰거나 '2'로 구독을 관리하세요.",
        )
        .block(Block::default().borders(Borders::ALL).title("게시글"))
        .wrap(Wrap { trim: true });
        empty.render(area, buf);
        return;
    }

    let items: Vec<ListItem<'_>> = posts
        .iter()
        .enumerate()
        .map(|(index, post)| {
            let marker =
                if state.feed_focus == FeedFocus::Posts && index == state.selected_post_index {
                    "> "
                } else {
                    "  "
                };
            let summary = format!(
                "{marker}{} | {} | {}",
                short_id(&post.object_id),
                short_id(&post.author_id),
                first_line(&post.text)
            );
            let style = if index == state.selected_post_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(summary)).style(style)
        })
        .collect();
    let mut list_state = ListState::default();
    list_state.select(Some(
        state.selected_post_index.min(posts.len().saturating_sub(1)),
    ));
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("게시글"));
    StatefulWidget::render(list, area, buf, &mut list_state);
}

fn selected_index(state: &AppState) -> Option<usize> {
    let visible_count = state.visible_feed_category_count();
    if state.screen == Screen::Feed && visible_count > 0 {
        Some(
            state
                .selected_category_index
                .min(visible_count.saturating_sub(1)),
        )
    } else {
        None
    }
}

fn short_id(id: &str) -> String {
    if id.len() > 12 {
        format!("{}...", &id[..12])
    } else {
        id.to_owned()
    }
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CategorySummary, Subscriptions};
    use chrono::Utc;

    #[test]
    fn feed_renders_posts_for_selected_category() {
        let mut state = AppState::new();
        state.categories = vec![CategorySummary {
            object_id: "oid".to_owned(),
            display_name: "General".to_owned(),
            description: String::new(),
            category_id: "cat-general".to_owned(),
            created_at: Utc::now(),
        }];
        state.subscriptions = Subscriptions {
            category_ids: vec!["cat-general".to_owned()],
        };
        state.posts.insert(
            "cat-general".to_owned(),
            vec![FeedPost {
                object_id: "post-1".to_owned(),
                author_id: "author-1".to_owned(),
                text: "Hello from the feed test".to_owned(),
                created_at: Utc::now(),
            }],
        );
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        render_feed(&state, buffer.area, &mut buffer);
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        let compact = text.replace(' ', "");
        assert!(compact.contains("카테고리"));
        assert!(text.contains("Hello from the feed test"));
    }

    #[test]
    fn feed_empty_state_wraps_full_korean_instruction() {
        let state = AppState::new();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        render_feed(&state, buffer.area, &mut buffer);
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        let compact = text.replace(' ', "");
        assert!(compact.contains("'n'으로글을"));
        assert!(compact.contains("쓰거나"));
        assert!(compact.contains("'2'로구독을관리하세요"));
    }
}
