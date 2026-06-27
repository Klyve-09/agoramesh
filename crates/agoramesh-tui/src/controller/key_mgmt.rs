//! Key-management action handlers.

use crate::app::{Action, AppState};
use crate::backend::Backend;
use crate::error::Error;
use crate::key_ux;
use crate::models::KeyStatus;

const KEY_OVERWRITE_DISABLED: &str = "Key overwrite disabled; use backup/restore instead";

/// Generates a development plaintext key.
pub(super) fn handle_generate_dev_key(backend: &Backend, state: &mut AppState) -> Option<Action> {
    match key_ux::generate_dev_key(backend) {
        Ok(key_status) => {
            state.key_status = key_status;
            state.key_input.status = Some("Development key generated".to_owned());
            state.status_message = Some("Development key generated".to_owned());
        }
        Err(error) => set_key_error_status(state, error.to_string()),
    }
    None
}

/// Generates and unlocks a passphrase-encrypted keyring.
pub(super) fn handle_generate_encrypted_key(
    backend: &Backend,
    state: &mut AppState,
) -> Option<Action> {
    if state.key_input.passphrase.is_empty() {
        let message = "type a passphrase before generating an encrypted key".to_owned();
        state.key_input.status = Some(message.clone());
        state.status_message = Some(message);
        return None;
    }
    match backend.generate_encrypted_key(&state.key_input.passphrase) {
        Ok(status) => {
            state.key_status = status;
            state.key_input.passphrase.clear();
            state.key_input.status = Some("Encrypted key generated and unlocked".to_owned());
            state.status_message = Some("Encrypted key generated and unlocked".to_owned());
        }
        Err(error) => set_key_error_status(state, error.to_string()),
    }
    None
}

/// Unlocks an existing encrypted keyring for this TUI session.
pub(super) fn handle_unlock_key(backend: &Backend, state: &mut AppState) -> Option<Action> {
    if state.key_input.passphrase.is_empty() {
        let message = "type a passphrase before unlocking the key".to_owned();
        state.key_input.status = Some(message.clone());
        state.status_message = Some(message);
        return None;
    }
    match backend.unlock_key(&state.key_input.passphrase) {
        Ok(status) => {
            state.key_status = status;
            state.key_input.passphrase.clear();
            state.key_input.status = Some("Encrypted key unlocked".to_owned());
            state.status_message = Some("Encrypted key unlocked".to_owned());
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
            let message = format!("Key backup written to {}", path.display());
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
                        format!("Key restored from backup: {public_key_hex}")
                    }
                    KeyStatus::Locked { public_key_hex } => {
                        let public_key = public_key_hex
                            .as_deref()
                            .unwrap_or("encrypted key restored");
                        format!("Key restored from backup: {public_key}")
                    }
                    KeyStatus::Missing => "Key restored from backup".to_owned(),
                };
                state.key_status = status;
                state.key_input.status = Some(message.clone());
                state.status_message = Some(message);
            }
            Err(error) => set_key_error_status(
                state,
                format!(
                    "Restore failed: key status could not be refreshed ({error}). Existing key was not changed."
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
    let message = if message == format!("message error: {KEY_OVERWRITE_DISABLED}") {
        KEY_OVERWRITE_DISABLED.to_owned()
    } else {
        message
    };
    state.key_input.status = Some(message.clone());
    state.status_message = Some(message);
}

fn backup_error_message(error: &Error) -> String {
    match error {
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::NotFound => {
            "Backup failed: no identity key exists yet. Generate or restore a key first.".to_owned()
        }
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::PermissionDenied => {
            "Backup failed: backup path is not writable. No key was changed.".to_owned()
        }
        Error::StateIo(_) => {
            "Backup failed: backup file could not be written. No key was changed.".to_owned()
        }
        _ => format!("Backup failed: {error}. No key was changed."),
    }
}

fn restore_error_message(error: &Error) -> String {
    match error {
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::NotFound => {
            "Restore failed: backup file is missing or unreadable. Existing key was not changed."
                .to_owned()
        }
        Error::StateIo(source) if source.kind() == std::io::ErrorKind::PermissionDenied => {
            "Restore failed: key path is not writable. Existing key was not changed.".to_owned()
        }
        Error::StateIo(_) => {
            "Restore failed: backup file could not be read or written. Existing key was not changed."
                .to_owned()
        }
        Error::StateJson(_) | Error::Key(_) => {
            "Restore failed: backup file is not a valid identity key. Existing key was not changed."
                .to_owned()
        }
        _ => format!("Restore failed: {error}. Existing key was not changed."),
    }
}
