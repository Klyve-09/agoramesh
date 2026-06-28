//! Key-management action handlers.

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::error::Error;
use crate::key_ux;
use crate::models::KeyStatus;

const KEY_OVERWRITE_DISABLED: &str = "키 덮어쓰기는 비활성화되어 있습니다. 백업/복원을 사용하세요";

/// Generates a development plaintext key.
pub(super) fn handle_generate_dev_key(backend: &Backend, state: &mut AppState) -> Option<Action> {
    match key_ux::generate_dev_key(backend) {
        Ok(key_status) => {
            state.key_status = key_status;
            state.key_input.status = Some("개발용 키가 생성되었습니다".to_owned());
            state.status_message = Some("개발용 키가 생성되었습니다".to_owned());
        }
        Err(error) => set_key_error_status(state, key_error_message(&error)),
    }
    None
}

/// Generates and unlocks a passphrase-encrypted keyring.
pub(super) fn handle_generate_encrypted_key(
    backend: &Backend,
    state: &mut AppState,
) -> Option<Action> {
    if state.key_input.passphrase.is_empty() {
        let message = "암호화 키를 생성하기 전에 암호구문을 입력하세요".to_owned();
        state.key_input.status = Some(message.clone());
        state.status_message = Some(message);
        return None;
    }
    match backend.generate_encrypted_key(&state.key_input.passphrase) {
        Ok(status) => {
            state.key_status = status;
            state.key_input.passphrase.clear();
            state.key_input.status = Some("암호화 키가 생성되고 잠금 해제되었습니다".to_owned());
            state.status_message = Some("암호화 키가 생성되고 잠금 해제되었습니다".to_owned());
        }
        Err(error) => set_key_error_status(state, key_error_message(&error)),
    }
    None
}

/// Unlocks an existing encrypted keyring for this TUI session.
pub(super) fn handle_unlock_key(backend: &Backend, state: &mut AppState) -> Option<Action> {
    if state.key_input.passphrase.is_empty() {
        let message = "키를 잠금 해제하기 전에 암호구문을 입력하세요".to_owned();
        state.key_input.status = Some(message.clone());
        state.status_message = Some(message);
        return None;
    }
    match backend.unlock_key(&state.key_input.passphrase) {
        Ok(status) => {
            state.key_status = status;
            state.key_input.passphrase.clear();
            state.key_input.status = Some("암호화 키가 잠금 해제되었습니다".to_owned());
            state.status_message = Some("암호화 키가 잠금 해제되었습니다".to_owned());
        }
        Err(error) => {
            let message = error.to_string();
            state.key_input.status = Some(message.clone());
            state.status_message = Some(message);
        }
    }
    None
}

/// Backs up the current identity key.
pub(super) fn handle_backup_key(backend: &Backend, state: &mut AppState) -> Option<Action> {
    match backend.backup_key() {
        Ok(path) => {
            let message = format!("키 백업을 {}에 썼습니다", path.display());
            state.key_input.status = Some(message.clone());
            state.status_message = Some(message);
        }
        Err(error) => set_key_error_status(state, backup_error_message(&error)),
    }
    None
}

/// Restores the identity key from the default backup copy.
pub(super) fn handle_restore_key(backend: &Backend, state: &mut AppState) -> Option<Action> {
    match backend.restore_key_from_backup() {
        Ok(()) => match backend.key_status(false) {
            Ok(status) => {
                let message = match &status {
                    KeyStatus::Present { public_key_hex } => {
                        format!("백업에서 키를 복원했습니다: {public_key_hex}")
                    }
                    KeyStatus::Locked { public_key_hex } => {
                        let public_key = public_key_hex.as_deref().unwrap_or("암호화 키 복원됨");
                        format!("백업에서 키를 복원했습니다: {public_key}")
                    }
                    KeyStatus::Missing => "백업에서 키를 복원했습니다".to_owned(),
                };
                state.key_status = status;
                state.key_input.status = Some(message.clone());
                state.status_message = Some(message);
            }
            Err(error) => set_key_error_status(
                state,
                format!(
                    "복원 실패: 키 상태를 새로고침할 수 없습니다({error}). 기존 키는 변경되지 않았습니다."
                ),
            ),
        },
        Err(error) => {
            if let Ok(status) = backend.key_status(false) {
                state.key_status = status;
            }
            set_key_error_status(state, restore_error_message(&error));
        }
    }
    None
}

fn set_key_error_status(state: &mut AppState, message: String) {
    state.key_input.status = Some(message.clone());
    state.status_message = Some(message);
}

fn key_error_message(error: &Error) -> String {
    match error {
        Error::Message(message) if message == KEY_OVERWRITE_DISABLED => {
            KEY_OVERWRITE_DISABLED.to_owned()
        }
        _ => error.to_string(),
    }
}

fn backup_error_message(error: &Error) -> String {
    match error {
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::NotFound => {
            "백업 실패: 아직 신원 키가 없습니다. 먼저 키를 생성하거나 복원하세요.".to_owned()
        }
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::PermissionDenied => {
            "백업 실패: 백업 경로에 쓸 수 없습니다. 키는 변경되지 않았습니다.".to_owned()
        }
        Error::StateIo(_) => {
            "백업 실패: 백업 파일을 쓸 수 없습니다. 키는 변경되지 않았습니다.".to_owned()
        }
        _ => format!("백업 실패: {error}. 키는 변경되지 않았습니다."),
    }
}

fn restore_error_message(error: &Error) -> String {
    match error {
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::NotFound => {
            "복원 실패: 백업 파일이 없거나 읽을 수 없습니다. 기존 키는 변경되지 않았습니다."
                .to_owned()
        }
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::PermissionDenied => {
            "복원 실패: 키 경로에 쓸 수 없습니다. 기존 키는 변경되지 않았습니다.".to_owned()
        }
        Error::StateIo(_) => {
            "복원 실패: 백업 파일을 읽거나 쓸 수 없습니다. 기존 키는 변경되지 않았습니다."
                .to_owned()
        }
        Error::StateJson(_) | Error::Key(_) => {
            "복원 실패: 백업 파일이 올바른 신원 키가 아닙니다. 기존 키는 변경되지 않았습니다."
                .to_owned()
        }
        _ => format!("복원 실패: {error}. 기존 키는 변경되지 않았습니다."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_overwrite_disabled_status_uses_typed_error_message() {
        let message = key_error_message(&Error::Message(KEY_OVERWRITE_DISABLED.to_owned()));

        assert_eq!(message, KEY_OVERWRITE_DISABLED);
        assert!(!message.starts_with("메시지 오류:"));
    }
}
