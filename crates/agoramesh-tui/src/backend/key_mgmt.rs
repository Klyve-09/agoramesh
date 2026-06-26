//! Key management operations for the TUI backend.

use std::path::PathBuf;

use agoramesh_cli::keyring::{self, Keyring};
use agoramesh_core::identity::Keypair;

use crate::backend::Backend;
use crate::backend::file_io::{remove_temp_file, sync_parent_dir, write_atomic, write_temp_file};
use crate::error::Error;
use crate::models::KeyStatus;

pub(super) fn key_status(backend: &Backend, generate_if_missing: bool) -> Result<KeyStatus, Error> {
    let path = backend.config.key_path();
    if !path.exists() && generate_if_missing {
        if backend.plaintext {
            Keyring::new(&path).dev_plaintext_save()?;
        } else {
            return Ok(KeyStatus::Missing);
        }
    }
    if !path.exists() {
        return Ok(KeyStatus::Missing);
    }
    if backend.plaintext {
        let keypair = load_keypair(backend)?;
        return Ok(KeyStatus::Present {
            public_key_hex: keyring::public_key_hex(&keypair),
        });
    }

    let passphrase = session_passphrase(backend)?;
    if let Some(passphrase) = passphrase {
        let keypair = Keyring::new(&path).load(&passphrase)?;
        return Ok(KeyStatus::Present {
            public_key_hex: keyring::public_key_hex(&keypair),
        });
    }

    Ok(KeyStatus::Locked {
        public_key_hex: keyring::read_encrypted_public_key_for_display(&path)?,
    })
}

pub(super) fn generate_dev_key(backend: &Backend) -> Result<KeyStatus, Error> {
    if !backend.plaintext {
        return Err(Error::Message(
            "plaintext key generation is only available in dev mode".to_owned(),
        ));
    }
    reject_key_overwrite(backend)?;
    Keyring::new(&backend.config.key_path()).dev_plaintext_save()?;
    key_status(backend, false)
}

pub(super) fn generate_encrypted_key(
    backend: &Backend,
    passphrase: &str,
) -> Result<KeyStatus, Error> {
    reject_key_overwrite(backend)?;
    Keyring::new(&backend.config.key_path()).generate(passphrase)?;
    set_session_passphrase(backend, passphrase)?;
    key_status(backend, false)
}

pub(super) fn unlock_key(backend: &Backend, passphrase: &str) -> Result<KeyStatus, Error> {
    let keypair = Keyring::new(&backend.config.key_path()).load(passphrase)?;
    set_session_passphrase(backend, passphrase)?;
    Ok(KeyStatus::Present {
        public_key_hex: keyring::public_key_hex(&keypair),
    })
}

pub(super) fn backup_key(backend: &Backend) -> Result<PathBuf, Error> {
    let source = backend.config.key_path();
    let backup = backup_key_path(backend);
    let bytes = std::fs::read(&source).map_err(Error::StateIo)?;
    write_atomic(&backup, &bytes)?;
    Ok(backup)
}

pub(super) fn restore_key_from_backup(backend: &Backend) -> Result<(), Error> {
    let bytes = std::fs::read(backup_key_path(backend)).map_err(Error::StateIo)?;
    validate_restored_key_bytes(backend, &bytes)?;
    let target = backend.config.key_path();
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(Error::StateIo)?;
    }
    let tmp_path = target.with_extension("restore.tmp");
    write_temp_file(&tmp_path, &bytes)?;
    if let Err(error) = validate_restored_key_file(backend, &tmp_path) {
        remove_temp_file(&tmp_path);
        return Err(error);
    }
    match std::fs::rename(&tmp_path, &target) {
        Ok(()) => {
            sync_parent_dir(&target);
            Ok(())
        }
        Err(source) => {
            remove_temp_file(&tmp_path);
            Err(Error::StateIo(source))
        }
    }
}

pub(super) fn reject_key_overwrite(backend: &Backend) -> Result<(), Error> {
    if backend.config.key_path().exists() {
        return Err(Error::Message(
            "Key overwrite disabled; use backup/restore instead".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn load_keypair(backend: &Backend) -> Result<Keypair, Error> {
    if backend.plaintext {
        return Ok(Keyring::new(&backend.config.key_path()).dev_plaintext_load()?);
    }
    let passphrase = session_passphrase(backend)?.ok_or_else(|| {
        Error::Message("encrypted key is locked; enter passphrase in Key Management".to_owned())
    })?;
    Ok(Keyring::new(&backend.config.key_path()).load(&passphrase)?)
}

pub(super) fn session_passphrase(backend: &Backend) -> Result<Option<String>, Error> {
    backend
        .passphrase
        .lock()
        .map_err(|_error| Error::Message("key session lock poisoned".to_owned()))
        .map(|passphrase| passphrase.clone())
}

pub(super) fn set_session_passphrase(backend: &Backend, passphrase: &str) -> Result<(), Error> {
    let mut session = backend
        .passphrase
        .lock()
        .map_err(|_error| Error::Message("key session lock poisoned".to_owned()))?;
    *session = Some(passphrase.to_owned());
    drop(session);
    Ok(())
}

fn backup_key_path(backend: &Backend) -> PathBuf {
    backend.config.data_dir.join("identity.key.backup")
}

fn validate_restored_key_bytes(backend: &Backend, bytes: &[u8]) -> Result<(), Error> {
    if backend.plaintext {
        keyring::validate_dev_plaintext_key_bytes_structure(bytes)?;
        return Ok(());
    }

    if let Some(passphrase) = session_passphrase(backend)? {
        keyring::load(bytes, &passphrase)?;
        return Ok(());
    }

    if backend.config.key_path().exists() {
        return Err(Error::Message(
            "encrypted key is locked; unlock before restoring over an existing key".to_owned(),
        ));
    }

    keyring::validate_encrypted_key_bytes_structure(bytes)?;
    Ok(())
}

fn validate_restored_key_file(backend: &Backend, path: &std::path::Path) -> Result<(), Error> {
    if backend.plaintext {
        Keyring::new(path).dev_plaintext_load()?;
        return Ok(());
    }

    if let Some(passphrase) = session_passphrase(backend)? {
        keyring::load_encrypted_key_with_passphrase(path, &passphrase)?;
        return Ok(());
    }

    keyring::validate_encrypted_key_file_structure(path)?;
    Ok(())
}
