//! Key management UX rendering for the TUI.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::app::AppState;
use crate::backend::Backend;
use crate::error::Error;
use crate::models::KeyStatus;

/// Renders the key management screen.
pub fn render_key_management(state: &AppState, area: Rect, buf: &mut Buffer) {
    let (title, lines) = match &state.key_status {
        KeyStatus::Missing => {
            let lines: Vec<Line<'_>> = vec![
                Line::from("신원 키를 찾을 수 없습니다."),
                Line::from(""),
                Line::from("암호문을 입력한 뒤 Ctrl+g로 암호화 키를 생성하세요."),
                Line::from("Ctrl+d는 --dev-insecure-plaintext-key로 시작했을 때만 사용하세요."),
                Line::from(""),
                Line::from(masked_passphrase_line(state)),
                Line::from(action_status_line(state)),
            ];
            ("키 관리 — 없음", lines)
        }
        KeyStatus::Locked { public_key_hex } => {
            let public_key = public_key_hex.as_deref().unwrap_or("암호화 키 있음");
            let lines: Vec<Line<'_>> = vec![
                Line::from("암호화된 신원 키가 잠겨 있습니다."),
                Line::from(format!("공개 키: {public_key}")),
                Line::from("암호문을 입력한 뒤 Enter로 잠금 해제하세요."),
                Line::from(""),
                Line::from(masked_passphrase_line(state)),
                Line::from(action_status_line(state)),
            ];
            ("키 관리 — 잠김", lines)
        }
        KeyStatus::Present { public_key_hex } => {
            let lines: Vec<Line<'_>> = vec![
                Line::from("신원 키가 있습니다."),
                Line::from(""),
                Line::from(format!("공개 키: {public_key_hex}")),
                Line::from(""),
                Line::from("백업 안내: 키 파일과 암호문을 안전하게 보관하세요."),
                Line::from("Ctrl+b로 identity.key.backup을 쓰고, Ctrl+r로 복원합니다."),
                Line::from("비밀 seed는 여기에서 절대 표시하지 않습니다."),
                Line::from(action_status_line(state)),
            ];
            ("키 관리 — 있음", lines)
        }
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .render(area, buf);
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
        "암호문: {}",
        "•".repeat(state.key_input.passphrase.chars().count())
    )
}

fn action_status_line(state: &AppState) -> String {
    state.key_input.status.clone().unwrap_or_else(|| {
        "키: Ctrl+g 암호화 생성 | Enter 잠금 해제 | Ctrl+b 백업 | Ctrl+r 복원 | Ctrl+d 개발용 평문"
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
        let compact = text.replace(' ', "");
        assert!(compact.contains("신원키가있습니다"));
        assert!(compact.contains("백업안내"));
        assert!(compact.contains("비밀seed는여기에서절대표시하지않습니다"));
        assert!(compact.contains("Ctrl+d"));
        assert!(compact.contains("개발용평문"));
    }
}
