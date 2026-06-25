#![allow(missing_docs, reason = "binary crate exposes no public API")]

use agoramesh_cli::commands;
use agoramesh_cli::config::Config;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "agoramesh", about = "Agoramesh mesh CLI")]
struct Cli {
    #[arg(long, value_name = "DIR", global = true)]
    data_dir: Option<PathBuf>,

    #[arg(long, value_name = "FILE", global = true)]
    key_path: Option<PathBuf>,

    #[arg(long, global = true)]
    dev_insecure_plaintext_key: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Category {
        #[command(subcommand)]
        command: commands::category::CategoryCommand,
    },
    Post {
        #[command(subcommand)]
        command: commands::post::PostCommand,
    },
    Comment {
        #[command(subcommand)]
        command: commands::comment::CommentCommand,
    },
    Feed {
        category_id: String,
        #[arg(long)]
        json: bool,
    },
    Sync {
        category_id: String,
        #[arg(long)]
        json: bool,
    },
    Run {
        #[arg(long, default_value = "127.0.0.1:0")]
        listen: SocketAddr,
        #[arg(long)]
        allow_public_bind: bool,
    },
    Peer {
        #[command(subcommand)]
        command: PeerCommand,
    },
    Key {
        #[command(subcommand)]
        command: commands::key::KeyCommand,
    },
}

#[derive(Debug, Subcommand)]
enum PeerCommand {
    Add {
        address: String,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Command::Category { command } => {
            let config = Config::open(cli.data_dir)?;
            let key_path = commands::helpers::resolve_key_path(&config, cli.key_path.as_deref());
            commands::category::run(command, &config, &key_path, cli.dev_insecure_plaintext_key)?;
        }
        Command::Post { command } => {
            let config = Config::open(cli.data_dir)?;
            let key_path = commands::helpers::resolve_key_path(&config, cli.key_path.as_deref());
            commands::post::run(command, &config, &key_path, cli.dev_insecure_plaintext_key)?;
        }
        Command::Comment { command } => {
            let config = Config::open(cli.data_dir)?;
            let key_path = commands::helpers::resolve_key_path(&config, cli.key_path.as_deref());
            commands::comment::run(command, &config, &key_path, cli.dev_insecure_plaintext_key)?;
        }
        Command::Feed { category_id, json } => {
            let config = Config::open(cli.data_dir)?;
            commands::feed::run(&config, &category_id, json)?;
        }
        Command::Sync { category_id, json } => {
            let config = Config::open(cli.data_dir)?;
            commands::sync::run(&config, &category_id, json).await?;
        }
        Command::Run {
            listen,
            allow_public_bind,
        } => {
            let config = Config::open(cli.data_dir)?;
            commands::run::run(&config, listen, allow_public_bind).await?;
        }
        Command::Peer { command } => {
            let config = Config::open(cli.data_dir)?;
            match command {
                PeerCommand::Add { address } => commands::peer::add(&config, &address)?,
                PeerCommand::List { json } => commands::peer::list(&config, json)?,
            }
        }
        Command::Key { command } => {
            let key_path = match cli.key_path {
                Some(path) => path,
                None => Config::open(cli.data_dir)?.key_path(),
            };
            commands::key::run(command, &key_path, cli.dev_insecure_plaintext_key)?;
        }
    }

    Ok(())
}
