//! DKLs Party CLI
//!
//! Command-line interface for running MPC party operations:
//! - Distributed Key Generation (DKG)
//! - Key Refresh
//! - Distributed Signature Generation (DSG)

use anyhow::Result;
use clap::{Parser, Subcommand};
use dkls23_core::{keygen, sign, KeyShare, SessionConfig};
use msg_relay_client::RelayClient;
use std::path::PathBuf;
use tracing::{info, Level};

/// DKLs Party - MPC Party Node
#[derive(Parser)]
#[command(name = "dkls-party")]
#[command(about = "Threshold ECDSA MPC party node")]
#[command(version)]
struct Cli {
    /// Relay service URL
    #[arg(short, long, env = "RELAY_URL", default_value = "http://127.0.0.1:8080")]
    relay: String,

    /// Party ID (0-indexed)
    #[arg(short, long, env = "PARTY_ID")]
    party_id: usize,

    /// Data directory for key shares
    #[arg(short, long, env = "DEST", default_value = "./data")]
    dest: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run distributed key generation
    Keygen {
        /// Number of parties
        #[arg(short, long)]
        n: usize,

        /// Threshold (t-of-n)
        #[arg(short, long)]
        t: usize,
    },

    /// Refresh key shares
    Refresh,

    /// Sign a message
    Sign {
        /// Message to sign (hex encoded hash)
        #[arg(short, long)]
        message: String,

        /// Participating party IDs (comma-separated)
        #[arg(short, long)]
        parties: String,
    },

    /// Derive a child key
    Derive {
        /// BIP32 derivation path (e.g., m/0/1/42)
        #[arg(short, long)]
        path: String,
    },

    /// Show key share info
    Info,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    // Ensure data directory exists
    std::fs::create_dir_all(&cli.dest)?;

    let relay = RelayClient::new(&cli.relay, cli.party_id);

    match cli.command {
        Commands::Keygen { n, t } => {
            run_keygen(&cli, &relay, n, t).await?;
        }
        Commands::Refresh => {
            run_refresh(&cli, &relay).await?;
        }
        Commands::Sign { ref message, ref parties } => {
            run_sign(&cli, &relay, message, parties).await?;
        }
        Commands::Derive { ref path } => {
            run_derive(&cli, path)?;
        }
        Commands::Info => {
            show_info(&cli)?;
        }
    }

    Ok(())
}

async fn run_keygen(cli: &Cli, relay: &RelayClient, n: usize, t: usize) -> Result<()> {
    info!(
        party_id = cli.party_id,
        n_parties = n,
        threshold = t,
        "Starting DKG"
    );

    let config = SessionConfig::new(n, t, cli.party_id)?;
    let key_share = keygen::run_dkg(&config, relay).await?;

    // Save key share
    let key_share_path = cli.dest.join(format!("keyshare.{}.json", cli.party_id));
    let json = serde_json::to_string_pretty(&key_share)?;
    std::fs::write(&key_share_path, json)?;

    info!(
        public_key = hex::encode(&key_share.public_key),
        path = ?key_share_path,
        "DKG completed, key share saved"
    );

    // Print public key
    println!("Public Key: {}", hex::encode(&key_share.public_key));

    Ok(())
}

async fn run_refresh(cli: &Cli, relay: &RelayClient) -> Result<()> {
    let key_share = load_key_share(cli)?;

    info!(
        party_id = cli.party_id,
        "Starting key refresh"
    );

    let config = SessionConfig::new(
        key_share.n_parties,
        key_share.threshold,
        cli.party_id,
    )?;

    let new_key_share = keygen::run_key_refresh(&config, &key_share, relay).await?;

    // Save new key share
    let key_share_path = cli.dest.join(format!("keyshare.{}.json", cli.party_id));
    let json = serde_json::to_string_pretty(&new_key_share)?;
    std::fs::write(&key_share_path, json)?;

    info!("Key refresh completed");

    Ok(())
}

async fn run_sign(
    cli: &Cli,
    relay: &RelayClient,
    message: &str,
    parties_str: &str,
) -> Result<()> {
    let key_share = load_key_share(cli)?;

    // Parse message (expected hex-encoded 32-byte hash)
    let message_bytes: [u8; 32] = hex::decode(message)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("Message must be 32 bytes"))?;

    // Parse parties
    let parties: Vec<usize> = parties_str
        .split(',')
        .map(|s| s.trim().parse())
        .collect::<std::result::Result<Vec<_>, _>>()?;

    info!(
        party_id = cli.party_id,
        participants = ?parties,
        message = message,
        "Starting DSG"
    );

    let signature = sign::run_dsg(&key_share, &message_bytes, &parties, relay).await?;

    info!(
        r = hex::encode(&signature.r),
        s = hex::encode(&signature.s),
        recovery_id = signature.recovery_id,
        "Signature generated"
    );

    // Print signature
    println!("Signature:");
    println!("  r: {}", hex::encode(&signature.r));
    println!("  s: {}", hex::encode(&signature.s));
    println!("  v: {}", signature.recovery_id);
    println!("  DER: {}", hex::encode(signature.to_der()));

    Ok(())
}

fn run_derive(cli: &Cli, path: &str) -> Result<()> {
    let key_share = load_key_share(cli)?;

    info!(
        party_id = cli.party_id,
        path = path,
        "Deriving child key"
    );

    let derived = key_share.derive_child(path)?;

    // Save derived key share
    let derived_path = cli.dest.join(format!(
        "keyshare.{}.derived.json",
        cli.party_id
    ));
    let json = serde_json::to_string_pretty(&derived)?;
    std::fs::write(&derived_path, json)?;

    info!(
        public_key = hex::encode(&derived.public_key),
        path = ?derived_path,
        "Child key derived and saved"
    );

    println!("Derived Public Key: {}", hex::encode(&derived.public_key));

    Ok(())
}

fn show_info(cli: &Cli) -> Result<()> {
    let key_share = load_key_share(cli)?;

    println!("Key Share Info:");
    println!("  Party ID: {}", key_share.party_id);
    println!("  N Parties: {}", key_share.n_parties);
    println!("  Threshold: {}", key_share.threshold);
    println!("  Public Key: {}", hex::encode(&key_share.public_key));
    println!("  Chain Code: {}", hex::encode(&key_share.chain_code));

    Ok(())
}

fn load_key_share(cli: &Cli) -> Result<KeyShare> {
    let key_share_path = cli.dest.join(format!("keyshare.{}.json", cli.party_id));
    let json = std::fs::read_to_string(&key_share_path)?;
    let key_share: KeyShare = serde_json::from_str(&json)?;
    Ok(key_share)
}
