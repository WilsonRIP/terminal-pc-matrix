//! src/main.rs
//! ────────────
//! Top-level CLI dispatcher.

mod cli;
mod file_ops;
mod browser_ops;
mod interactive;
mod utils;
mod network_ops;
mod http_ops;
mod dns_ops;

use clap::Parser;
use colored::*;
use cli::{Cli, Commands};
use std::process::exit;

/// Tokio runtime: a multithreaded scheduler is the default; specify the flavour
/// explicitly to make intent clear.
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // One central error handler with colourised output.
    if let Err(err) = async_main().await {
        eprintln!("{} {}", "⛔  Error:".red().bold(), err);
        exit(1);
    }
}

async fn async_main() -> anyhow::Result<()> {
    // Nice back-traces & colourised eyre reports (optional)
    color_eyre::install().ok();

    let cli_args = Cli::parse();

    match cli_args.command {
        // ─────────────────────────────── FILE OPS ───────────────────────────────
        Some(Commands::List { path })                       => file_ops::list_directory(&path)?,
        Some(Commands::Backup { source, destination })      => file_ops::backup_directory(&source, &destination)?,
        Some(Commands::OrganizeScreenshots)                 => file_ops::organize_screenshots().map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::AnalyzeDisk { path, top })           => file_ops::analyze_disk(&path, top).map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::CleanSystem { dry_run })             => file_ops::clean_system(dry_run).map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::Rename(args))                        => file_ops::rename_files(&args).map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::FindDuplicates { path, min_size })   => file_ops::find_duplicates(&path, &min_size).map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::SyncFolders(args))                   => file_ops::sync_folders(&args).map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::SearchFiles { path, query })         => file_ops::search_files(&path, &query).map_err(|e| anyhow::anyhow!("{}", e))?,

        // ─────────────────────────────── SYSTEM OPS ─────────────────────────────
        Some(Commands::CloseBrowsers)                       => browser_ops::close_browsers().map_err(|e| anyhow::anyhow!("{}", e))?,

        // ─────────────────────────────── NETWORK OPS ────────────────────────────
        Some(Commands::Bandwidth {})                        => network_ops::discover_network_devices(350).await.map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::PortScan(args))                      => {
            network_ops::scan_ports(&args.host, &args.ports, args.timeout).await.map_err(|e| anyhow::anyhow!("{}", e))?
        }

        // ─────────────────────────────── HTTP / DNS / NETWORK ─────────────────────
        Some(Commands::HttpRequest(args)) => {
            let headers = args.headers.into_iter().collect();
            http_ops::make_request(&args.method, &args.url, args.body.as_deref(), &headers).await.map_err(|e| anyhow::anyhow!("{}", e))?
        }
        Some(Commands::DnsCache(args))                      => dns_ops::manage_dns(args.action).await.map_err(|e| anyhow::anyhow!("{}", e))?,
        Some(Commands::Ping(args))                          => network_ops::ping_host(&args.host, args.count).await.map_err(|e| anyhow::anyhow!("{}", e))?,

        // ─────────────────────────────── INTERACTIVE ────────────────────────────
        None                                                => interactive::run_interactive_mode().await.map_err(|e| anyhow::anyhow!("{}", e))?,
    }

    Ok(())
}
