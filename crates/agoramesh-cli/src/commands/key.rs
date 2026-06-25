//! Key command handlers.

#![allow(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "CLI commands intentionally write human-readable output to the terminal"
)]

use std::io::BufRead;
use std::path::Path;

use agoramesh_core::Keypair;
use clap::{Args, Subcommand};

use crate::keyring::{self, Keyring};

/// Key management subcommands.
#[derive(Debug, Subcommand)]
pub enum KeyCommand {
    /// Generate a new mesh identity key.
    Generate,
    /// Show the public identity for an existing key.
    Show(ShowArgs),
}

/// Arguments for `key show`.
#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Export the secret seed alongside the public identity.
    #[arg(long)]
    pub show_secret: bool,
}

/// Runs a key subcommand.
pub fn run(command: KeyCommand, key_path: &Path, plaintext: bool) -> Result<(), Error> {
    match command {
        KeyCommand::Generate => generate(key_path, plaintext),
        KeyCommand::Show(args) => show(key_path, plaintext, args.show_secret),
    }
}

fn generate(key_path: &Path, plaintext: bool) -> Result<(), Error> {
    if plaintext {
        Keyring::new(key_path).dev_plaintext_save()?;
        eprintln!("wrote development plaintext key to {}", key_path.display());
        return Ok(());
    }

    let passphrase = read_new_passphrase()?;
    Keyring::new(key_path).generate(&passphrase)?;
    eprintln!("wrote encrypted key to {}", key_path.display());
    Ok(())
}

fn show(key_path: &Path, plaintext: bool, show_secret: bool) -> Result<(), Error> {
    let keypair = load_keypair(key_path, plaintext)?;
    println!("identity: {}", identity_hex(&keypair));
    if show_secret {
        eprintln!("WARNING: exporting secret seed; keep it private");
        println!("secret_seed: {}", keypair.to_base64());
    }
    Ok(())
}

pub(crate) fn load_keypair(key_path: &Path, plaintext: bool) -> Result<Keypair, Error> {
    if plaintext {
        return Ok(Keyring::new(key_path).dev_plaintext_load()?);
    }
    let passphrase = read_existing_passphrase()?;
    Ok(Keyring::new(key_path).load(&passphrase)?)
}

fn read_new_passphrase() -> Result<String, Error> {
    if console::user_attended() {
        return dialoguer::Password::new()
            .with_prompt("Key passphrase")
            .with_confirmation("Repeat key passphrase", "passphrases do not match")
            .interact()
            .map_err(|error| Error::Prompt(error.to_string()));
    }

    let passphrase = read_stdin_line("Key passphrase")?;
    let confirmation = read_stdin_line("Repeat key passphrase")?;
    if passphrase != confirmation {
        return Err(Error::PassphraseMismatch);
    }
    Ok(passphrase)
}

fn read_existing_passphrase() -> Result<String, Error> {
    if console::user_attended() {
        return dialoguer::Password::new()
            .with_prompt("Key passphrase")
            .interact()
            .map_err(|error| Error::Prompt(error.to_string()));
    }

    read_stdin_line("Key passphrase")
}

fn read_stdin_line(prompt: &str) -> Result<String, Error> {
    eprintln!("{prompt}:");
    let mut input = String::new();
    std::io::stdin().lock().read_line(&mut input)?;
    let passphrase = input.trim_end_matches(['\r', '\n']).to_owned();
    if passphrase.is_empty() {
        return Err(Error::EmptyPassphrase);
    }
    Ok(passphrase)
}

fn identity_hex(keypair: &Keypair) -> String {
    hex::encode(keypair.identity().as_bytes())
}

/// Errors raised by the key command surface.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The user submitted two different passphrases while generating a key.
    #[error("passphrases do not match")]
    PassphraseMismatch,

    /// An empty passphrase was submitted.
    #[error("passphrase must not be empty")]
    EmptyPassphrase,

    /// A terminal prompt failed.
    #[error("passphrase prompt failed: {0}")]
    Prompt(String),

    /// Key file encoding, encryption, or I/O failed.
    #[error(transparent)]
    Keyring(#[from] keyring::KeyringError),

    /// Reading passphrase input from stdin failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
