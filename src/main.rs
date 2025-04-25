// Declare modules
mod cli;
mod file_ops;
mod browser_ops;
mod interactive;
mod utils;
mod network_ops;
mod http_ops;
mod dns_ops;

// Use items from modules
use cli::{Cli, Commands};
use clap::Parser;
use std::process::exit;
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_args = Cli::parse();

    let result = match cli_args.command {
        Some(Commands::List { path }) => file_ops::list_directory(&path).map_err(|e| e.into()),
        Some(Commands::Backup { source, destination }) => file_ops::backup_directory(&source, &destination).map_err(|e| e.into()),
        Some(Commands::CloseBrowsers) => browser_ops::close_browsers(),
        Some(Commands::OrganizeScreenshots) => file_ops::organize_screenshots(),
        Some(Commands::AnalyzeDisk { path, top }) => file_ops::analyze_disk(&path, top),
        Some(Commands::CleanSystem { dry_run }) => file_ops::clean_system(dry_run),
        Some(Commands::Rename(args)) => file_ops::rename_files(&args),
        Some(Commands::FindDuplicates { path, min_size }) => file_ops::find_duplicates(&path, &min_size),
        Some(Commands::SyncFolders(args)) => file_ops::sync_folders(&args),
        Some(Commands::SearchFiles { path, query }) => file_ops::search_files(&path, &query),
        Some(Commands::Bandwidth {}) => network_ops::get_bandwidth_snapshot().await,
        Some(Commands::PortScan(args)) => network_ops::scan_ports(&args.host, &args.ports, args.timeout).await,
        Some(Commands::HttpRequest(args)) => {
            let headers_map: std::collections::HashMap<String, String> = args.headers.into_iter().collect();
            http_ops::make_request(&args.method, &args.url, args.body.as_deref(), &headers_map).await
        },
        Some(Commands::DnsCache(args)) => dns_ops::manage_dns(args.action).await,
        None => {
            // No command provided, enter interactive mode
            interactive::run_interactive_mode().await
        }
    };

    if let Err(e) = result {
        eprintln!("{}: {}", "Error".red().bold(), e);
        exit(1);
    }

    Ok(())
} 