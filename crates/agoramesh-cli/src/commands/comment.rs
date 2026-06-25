#![allow(missing_docs, reason = "CLI command surface is described by clap help")]
#![allow(clippy::print_stdout, reason = "CLI output")]

use std::path::Path;

use agoramesh_core::objects::{ParentKind, comment};
use agoramesh_core::{MessageId, SystemClock};
use agoramesh_store::Store;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::category::{format_timestamp, parse_created_at};
use crate::commands::helpers;
use crate::config::Config;

#[derive(Debug, Subcommand)]
pub enum CommentCommand {
    Create(CommentCreateArgs),
}

#[derive(Debug, Args)]
pub struct CommentCreateArgs {
    #[arg(long)]
    pub category_id: String,
    #[arg(long, value_parser = parse_parent_kind)]
    pub parent_kind: ParentKind,
    #[arg(long)]
    pub parent_id: String,
    #[arg(long)]
    pub text: String,
    #[arg(long)]
    pub created_at: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct CreateOutput<'a> {
    object_id: &'a str,
    category_id: &'a str,
    kind: &'a str,
    created_at: String,
}

pub fn run(
    command: CommentCommand,
    config: &Config,
    key_path: &Path,
    plaintext: bool,
) -> Result<(), Error> {
    match command {
        CommentCommand::Create(args) => create(config, key_path, plaintext, args),
    }
}

fn create(
    config: &Config,
    key_path: &Path,
    plaintext: bool,
    args: CommentCreateArgs,
) -> Result<(), Error> {
    let created_at = parse_created_at(args.created_at.as_deref())?;
    let parent_id = MessageId::from_hex(&args.parent_id)?;
    let keypair = helpers::load_keypair(key_path, plaintext)?;
    let message = comment::create(
        &keypair,
        &args.category_id,
        args.parent_kind,
        parent_id,
        args.text,
        created_at,
    )?;
    let object_id = message.id().to_hex();
    let clock = SystemClock;
    let mut store = helpers::open_store(config)?;
    let _ = store.insert(message, &clock)?;

    if args.json {
        let output = CreateOutput {
            object_id: &object_id,
            category_id: &args.category_id,
            kind: "comment",
            created_at: format_timestamp(created_at),
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("{object_id}");
    }
    Ok(())
}

fn parse_parent_kind(value: &str) -> Result<ParentKind, String> {
    match value {
        "post" => Ok(ParentKind::Post),
        "comment" => Ok(ParentKind::Comment),
        other => Err(format!("parent kind must be post or comment, got {other}")),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Helpers(#[from] helpers::Error),
    #[error(transparent)]
    Category(#[from] crate::commands::category::Error),
    #[error(transparent)]
    Message(#[from] agoramesh_core::message::Error),
    #[error(transparent)]
    MessageId(#[from] agoramesh_core::message::MessageIdHexError),
    #[error(transparent)]
    Store(#[from] agoramesh_store::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
