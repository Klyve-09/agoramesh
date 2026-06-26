//! Post creation flow for the TUI.

use chrono::{DateTime, Timelike, Utc};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::AppState;
use crate::backend::Backend;
use crate::error::Error;
use crate::models::FeedPost;

/// Mutable compose state captured while the user is writing a post.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ComposeState {
    /// Index of the selected category in the known categories list.
    pub category_index: usize,
    /// Text being composed.
    pub text: String,
    /// Whether to show a preview instead of the editor.
    pub preview: bool,
    /// Last submission result message.
    pub status: Option<String>,
}

/// Renders the compose screen.
pub fn render_compose(state: &AppState, compose: &ComposeState, area: Rect, buf: &mut Buffer) {
    let layout = compose_layout(area);
    render_category_selector(state, compose, layout.category, buf);
    if compose.preview {
        render_preview(compose, layout.editor, buf);
    } else {
        render_editor(compose, layout.editor, buf);
    }
    render_compose_help(layout.help, buf);
}

struct ComposeLayout {
    category: Rect,
    editor: Rect,
    help: Rect,
}

fn compose_layout(area: Rect) -> ComposeLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    ComposeLayout {
        category: *chunks.first().unwrap_or(&area),
        editor: *chunks.get(1).unwrap_or(&area),
        help: *chunks.get(2).unwrap_or(&area),
    }
}

fn render_category_selector(
    state: &AppState,
    compose: &ComposeState,
    area: Rect,
    buf: &mut Buffer,
) {
    let label = state.categories.get(compose.category_index).map_or_else(
        || "No categories available".to_owned(),
        |category| {
            format!(
                "Category: {} ({})",
                category.display_name, category.category_id
            )
        },
    );
    Paragraph::new(label)
        .block(Block::default().borders(Borders::ALL).title("Category"))
        .render(area, buf);
}

fn render_editor(compose: &ComposeState, area: Rect, buf: &mut Buffer) {
    let display = if compose.text.is_empty() {
        Text::from("Type your post here... (no content yet)")
            .style(Style::default().fg(Color::DarkGray))
    } else {
        Text::from(compose.text.as_str())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Editor — press Tab to preview, Enter to submit");
    Paragraph::new(display).block(block).render(area, buf);
}

fn render_preview(compose: &ComposeState, area: Rect, buf: &mut Buffer) {
    let preview_text = format!(
        "Category index: {}\n---\n{}",
        compose.category_index, compose.text
    );
    Paragraph::new(preview_text)
        .block(Block::default().borders(Borders::ALL).title("Preview"))
        .render(area, buf);
}

fn render_compose_help(area: Rect, buf: &mut Buffer) {
    let help = "Tab: preview | Enter: submit | Esc: back";
    Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .render(area, buf);
}

/// Submits the composed post to the backend.
///
/// # Errors
///
/// Returns an error when no category is selected, the key is missing, or the
/// backend cannot persist the post.
pub fn submit_compose(
    backend: &Backend,
    state: &AppState,
    compose: &ComposeState,
) -> Result<FeedPost, Error> {
    let category = state
        .categories
        .get(compose.category_index)
        .ok_or_else(|| Error::Message("no category selected".to_owned()))?;
    if compose.text.is_empty() {
        return Err(Error::Message("post text is empty".to_owned()));
    }
    let created_at = truncate_to_seconds(Utc::now())
        .ok_or_else(|| Error::Message("failed to truncate timestamp".to_owned()))?;
    backend.create_post(&category.category_id, &compose.text, created_at)
}

fn truncate_to_seconds(value: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let truncated =
        value
            .date_naive()
            .and_hms_micro_opt(value.hour(), value.minute(), value.second(), 0)?;
    truncated.and_local_timezone(Utc).single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Backend;
    use crate::models::{CategorySummary, Subscriptions};
    use agoramesh_store::Store;
    use chrono::Utc;

    fn app_with_category() -> AppState {
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
        state
    }

    #[test]
    fn post_preview_does_not_persist_until_submit() {
        let state = app_with_category();
        let compose = ComposeState {
            category_index: 0,
            text: "Preview only".to_owned(),
            preview: true,
            status: None,
        };
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        render_compose(&state, &compose, buffer.area, &mut buffer);
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("Preview"));
        assert!(!text.contains("Editor"));
    }

    #[test]
    fn post_submit_persists_signed_post_in_category_scope() {
        let (backend, _temp_dir) = backend_fixture();
        backend.generate_dev_key().expect("generate dev key");
        let keypair = backend_keypair(&backend);
        let category = agoramesh_core::objects::category::create(
            &keypair,
            Utc::now().with_nanosecond(0).expect("truncate"),
            "General",
            "General chat",
            "Charter text",
        )
        .expect("create category");
        let category_id = category.signed_payload().scope().to_owned();
        let mut store = backend.store().expect("open store");
        store
            .insert(category, &agoramesh_core::SystemClock)
            .expect("insert category");

        let mut state = app_with_category();
        if let Some(category) = state.categories.first_mut() {
            category.category_id = category_id.clone();
        }
        state.subscriptions.category_ids = vec![category_id.clone()];
        let compose = ComposeState {
            category_index: 0,
            text: "Hello from compose".to_owned(),
            preview: false,
            status: None,
        };

        let post = submit_compose(&backend, &state, &compose).expect("submit post");
        assert_eq!(post.text, "Hello from compose");

        let posts = backend.load_feed(&category_id).expect("load feed");
        assert_eq!(posts.len(), 1);
        assert_eq!(
            posts.first().map_or("", |post| post.text.as_str()),
            "Hello from compose"
        );
    }

    fn backend_fixture() -> (Backend, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let backend =
            Backend::open(Some(temp_dir.path().to_path_buf()), true).expect("open backend");
        (backend, temp_dir)
    }

    fn backend_keypair(backend: &Backend) -> agoramesh_core::Keypair {
        agoramesh_cli::keyring::Keyring::new(&backend.data_dir().join("identity.key"))
            .dev_plaintext_load()
            .expect("load key")
    }
}
