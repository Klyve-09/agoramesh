//! Feed view model and rendering for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
};

use crate::app::AppState;
use crate::models::{FeedPost, Screen};

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
    let items: Vec<ListItem<'_>> = state
        .categories
        .iter()
        .enumerate()
        .map(|(index, category)| {
            let subscribed = state
                .subscriptions
                .category_ids
                .contains(&category.category_id);
            let marker = if subscribed { "* " } else { "  " };
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
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Categories"));
    StatefulWidget::render(list, area, buf, &mut list_state);
}

fn render_post_list(state: &AppState, area: Rect, buf: &mut Buffer) {
    let selected_category = selected_index(state).and_then(|index| state.categories.get(index));
    let posts: Vec<FeedPost> = selected_category
        .and_then(|category| state.posts.get(&category.category_id).cloned())
        .unwrap_or_default();

    if posts.is_empty() {
        let empty = Paragraph::new("No posts in this category. Press 'n' to compose.")
            .block(Block::default().borders(Borders::ALL).title("Posts"));
        empty.render(area, buf);
        return;
    }

    let items: Vec<ListItem<'_>> = posts
        .iter()
        .map(|post| {
            let summary = format!(
                "{} | {} | {}",
                short_id(&post.object_id),
                short_id(&post.author_id),
                first_line(&post.text)
            );
            ListItem::new(Line::from(summary))
        })
        .collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Posts"));
    Widget::render(list, area, buf);
}

fn selected_index(state: &AppState) -> Option<usize> {
    if state.screen == Screen::Feed && !state.categories.is_empty() {
        Some(
            state
                .selected_index
                .min(state.categories.len().saturating_sub(1)),
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
        assert!(text.contains("Categories"));
        assert!(text.contains("Hello from the feed test"));
    }
}
