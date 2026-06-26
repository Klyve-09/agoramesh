//! Local category subscription persistence for the TUI.

use std::path::Path;

use crate::error::Error;
use crate::models::Subscriptions;

/// Loads subscriptions from a JSON file, returning an empty set when absent.
///
/// # Errors
///
/// Returns an error when the file cannot be read or parsed.
pub fn load(path: &Path) -> Result<Subscriptions, Error> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(Error::StateJson),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(Subscriptions::default())
        }
        Err(source) => Err(Error::StateIo(source)),
    }
}

/// Saves subscriptions to a JSON file.
///
/// # Errors
///
/// Returns an error when the file cannot be written.
pub fn save(path: &Path, subscriptions: &Subscriptions) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::StateIo)?;
    }
    let bytes = serde_json::to_vec_pretty(subscriptions).map_err(Error::StateJson)?;
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, bytes).map_err(Error::StateIo)?;
    std::fs::rename(tmp_path, path).map_err(Error::StateIo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriptions_add_remove_and_reload() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let path = temp_dir.path().join("subscriptions.json");
        let mut subscriptions = Subscriptions::default();
        subscriptions.category_ids.push("cat-a".to_owned());
        subscriptions.category_ids.push("cat-b".to_owned());

        save(&path, &subscriptions).expect("save subscriptions");
        let loaded = load(&path).expect("load subscriptions");

        assert_eq!(
            loaded.category_ids,
            vec!["cat-a".to_owned(), "cat-b".to_owned()]
        );

        subscriptions
            .category_ids
            .retain(|category_id| category_id != "cat-a");
        save(&path, &subscriptions).expect("save subscriptions");
        let loaded = load(&path).expect("reload subscriptions");
        assert_eq!(loaded.category_ids, vec!["cat-b".to_owned()]);
    }

    #[test]
    fn subscriptions_missing_file_is_empty() {
        let temp_dir = tempfile::tempdir().expect("create tempdir");
        let path = temp_dir.path().join("missing.json");
        let loaded = load(&path).expect("load missing subscriptions");
        assert!(loaded.category_ids.is_empty());
    }
}
