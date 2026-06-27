#![allow(missing_docs, reason = "CLI command surface is described by clap help")]
#![allow(clippy::print_stdout, reason = "CLI output")]

use std::path::Path;

use agoramesh_core::SystemClock;
use agoramesh_core::objects::{acceptance, category};
use agoramesh_store::Store;
use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::helpers;
use crate::config::Config;

#[derive(Debug, Subcommand)]
pub enum CategoryCommand {
    Create(CategoryCreateArgs),
}

#[derive(Debug, Args)]
pub struct CategoryCreateArgs {
    #[arg(long)]
    pub display_name: String,
    #[arg(long)]
    pub description: String,
    #[arg(long)]
    pub charter: String,
    #[arg(long)]
    pub created_at: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct CategoryCreateOutput<'a> {
    object_id: &'a str,
    category_id: &'a str,
    kind: &'a str,
    created_at: String,
}

pub fn run(
    command: CategoryCommand,
    config: &Config,
    key_path: &Path,
    plaintext: bool,
) -> Result<(), Error> {
    match command {
        CategoryCommand::Create(args) => create(config, key_path, plaintext, args),
    }
}

fn create(
    config: &Config,
    key_path: &Path,
    plaintext: bool,
    args: CategoryCreateArgs,
) -> Result<(), Error> {
    let created_at = parse_created_at(args.created_at.as_deref())?;
    let keypair = helpers::load_keypair(key_path, plaintext)?;
    let message = category::create(
        &keypair,
        created_at,
        args.display_name,
        args.description,
        args.charter,
    )?;
    let object_id = message.id().to_hex();
    let category_id = message.signed_payload().scope().to_owned();
    let clock = SystemClock;
    helpers::ensure_phase1_acceptable(&message, &clock)?;
    let mut store = helpers::open_store(config)?;
    let _ = store.insert(message, &clock)?;

    if args.json {
        let output = CategoryCreateOutput {
            object_id: &object_id,
            category_id: &category_id,
            kind: "category",
            created_at: format_timestamp(created_at),
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("{category_id}");
    }
    Ok(())
}

pub(crate) fn parse_created_at(value: Option<&str>) -> Result<DateTime<Utc>, Error> {
    match value {
        Some(raw) => {
            let timestamp = DateTime::parse_from_rfc3339(raw)?.with_timezone(&Utc);
            if timestamp.timestamp_subsec_nanos() != 0 {
                return Err(Error::TimestampPrecision);
            }
            Ok(timestamp)
        }
        None => Ok(truncate_to_seconds(Utc::now())),
    }
}

fn truncate_to_seconds(value: DateTime<Utc>) -> DateTime<Utc> {
    value.with_nanosecond(0).unwrap_or(value)
}

pub(crate) fn format_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Helpers(#[from] helpers::Error),
    #[error(transparent)]
    Message(#[from] agoramesh_core::message::Error),
    #[error(transparent)]
    Store(#[from] agoramesh_store::Error),
    #[error("invalid RFC3339 timestamp: {0}")]
    CreatedAt(#[from] chrono::ParseError),
    #[error("created_at must use UTC RFC3339 seconds precision")]
    TimestampPrecision,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("object validation failed: {0}")]
    Acceptance(#[from] acceptance::Error),
}
