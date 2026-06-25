#![allow(missing_docs, reason = "CLI command surface is described by clap help")]
#![allow(clippy::print_stdout, reason = "CLI output")]

use agoramesh_core::{Message, SystemClock};
use agoramesh_store::Store;
use serde::Serialize;
use serde_json::Value;

use crate::commands::category::format_timestamp;
use crate::commands::helpers;
use crate::config::Config;

#[derive(Serialize)]
struct FeedItem {
    object_id: String,
    kind: String,
    created_at: String,
    body_json: Value,
}

pub fn run(config: &Config, category_id: &str, json: bool) -> Result<(), Error> {
    let store = helpers::open_store(config)?;
    let clock = SystemClock;
    let messages = store.list_by_scope(category_id, &clock)?;
    if json {
        let items = messages
            .iter()
            .map(feed_item)
            .collect::<Result<Vec<_>, Error>>()?;
        println!("{}", serde_json::to_string(&items)?);
    } else {
        for message in messages {
            let signed = message.signed_payload();
            println!(
                "{} [{}] {}",
                format_timestamp(signed.created_at().datetime()),
                signed.kind(),
                message.id().to_hex()
            );
        }
    }
    Ok(())
}

fn feed_item(message: &Message) -> Result<FeedItem, Error> {
    let signed = message.signed_payload();
    Ok(FeedItem {
        object_id: message.id().to_hex(),
        kind: signed.kind().to_owned(),
        created_at: format_timestamp(signed.created_at().datetime()),
        body_json: serde_json::from_slice(signed.body())?,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Helpers(#[from] helpers::Error),
    #[error(transparent)]
    Store(#[from] agoramesh_store::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
