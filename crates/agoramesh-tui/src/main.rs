//! Binary entry point for the `AgoraMesh` TUI.

use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;

use agoramesh_tui::terminal::run;

#[derive(Debug, Parser)]
#[command(name = "agoramesh-tui")]
#[command(about = "Minimal terminal UI for AgoraMesh")]
struct Args {
    /// Data directory for keys, store, peers, and TUI state.
    #[arg(long, env = "AGORAMESH_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Run in plaintext dev key mode (do not use for real identities).
    #[arg(long, env = "AGORAMESH_PLAINTEXT")]
    plaintext: bool,

    /// Allow the TUI background sync server to bind to a public address.
    #[arg(long, env = "AGORAMESH_ALLOW_PUBLIC_BIND")]
    allow_public_bind: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    run(args.data_dir, args.plaintext, args.allow_public_bind)?;
    Ok(())
}
