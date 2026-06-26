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
    #[arg(
        long,
        alias = "plaintext",
        env = "AGORAMESH_DEV_INSECURE_PLAINTEXT_KEY"
    )]
    dev_insecure_plaintext_key: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    run(args.data_dir, args.dev_insecure_plaintext_key)?;
    Ok(())
}
