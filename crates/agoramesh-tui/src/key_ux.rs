//! Key management UX rendering for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::AppState;
use crate::backend::Backend;
use crate::error::Error;
use crate::models::KeyStatus;

/// Renders the key management screen.
pub fn render_key_management(state: &AppState, area: Rect, buf: &mut Buffer) {
    let (title, lines) = match &state.key_status {
        KeyStatus::Missing => {
            let lines: Vec<Line<'_>> = vec![
                Line::from("No identity key found."),
                Line::from(""),
                Line::from("Type a passphrase, then press g to generate an encrypted key."),
                Line::from("Use d only when started with --dev-insecure-plaintext-key."),
                Line::from(""),
                Line::from(masked_passphrase_line(state)),
                Line::from(action_status_line(state)),
            ];
            ("Key Management — Missing", lines)
        }
        KeyStatus::Locked { public_key_hex } => {
            let public_key = public_key_hex.as_deref().unwrap_or("encrypted key present");
            let lines: Vec<Line<'_>> = vec![
                Line::from("Encrypted identity key is locked."),
                Line::from(format!("Public key: {public_key}")),
                Line::from("Type passphrase, then press u or Enter to unlock."),
                Line::from(""),
                Line::from(masked_passphrase_line(state)),
                Line::from(action_status_line(state)),
            ];
            ("Key Management — Locked", lines)
        }
        KeyStatus::Present { public_key_hex } => {
            let lines: Vec<Line<'_>> = vec![
                Line::from("Identity key present."),
                Line::from(""),
                Line::from(format!("Public key: {public_key_hex}")),
                Line::from(""),
                Line::from("Backup hint: keep your key file and passphrase safe."),
                Line::from("Press Ctrl+b to write identity.key.backup; Ctrl+r restores it."),
                Line::from("The secret seed is never shown here."),
                Line::from(action_status_line(state)),
            ];
            ("Key Management — Present", lines)
        }
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    Paragraph::new(lines).block(block).render(area, buf);
}

/// Generates a development plaintext key when the backend is in plaintext mode.
///
/// # Errors
///
/// Returns an error when the backend refuses to generate a plaintext key or
/// the key file cannot be written.
pub fn generate_dev_key(backend: &Backend) -> Result<KeyStatus, Error> {
    backend.generate_dev_key()
}

fn masked_passphrase_line(state: &AppState) -> String {
    format!(
        "Passphrase: {}",
        "•".repeat(state.key_input.passphrase.chars().count())
    )
}

fn action_status_line(state: &AppState) -> String {
    state.key_input.status.clone().unwrap_or_else(|| {
        "Keys: Ctrl+g generate encrypted | Enter unlock | Ctrl+b backup | Ctrl+r restore | Ctrl+d dev plaintext"
            .to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Backend;

    fn backend_fixture() -> (Backend, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let backend =
            Backend::open(Some(temp_dir.path().to_path_buf()), true).expect("open backend");
        (backend, temp_dir)
    }

    #[test]
    fn key_panel_generates_key_and_shows_backup_hints() {
        let (backend, _temp_dir) = backend_fixture();
        let status = generate_dev_key(&backend).expect("generate key");
        assert!(matches!(status, KeyStatus::Present { .. }));

        let mut state = crate::app::AppState::new();
        state.key_status = status;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        render_key_management(&state, buffer.area, &mut buffer);
        let text = buffer
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("Identity key present"));
        assert!(text.contains("Backup hint"));
        assert!(text.contains("secret seed is never shown here"));
    }
}
