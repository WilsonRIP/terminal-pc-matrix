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
mod calculator_ops;
mod unit_converter_ops;
mod whois_ops;
mod ip_info_ops;
mod file_download_ops;
mod video_download_ops;
mod image_download_ops;
mod antivirus_ops;
mod pc_specs_ops;

use clap::Parser;
use colored::*;
use cli::{Cli, Commands};
use std::process::exit;
use std::path::PathBuf;
use crate::unit_converter_ops::handle_unit_converter_command;

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

        // ─────────────────────────────── UNIT CONVERTER ─────────────────────────
        Some(Commands::Convert(args)) => {
            match handle_unit_converter_command(args) {
                Ok(output) => println!("{}", output),
                Err(e) => eprintln!("Error during conversion: {}", e),
            }
        }

        // ─────────────────────────────── WHOIS LOOKUP ───────────────────────────
        Some(Commands::Whois(args)) => {
            match whois_ops::lookup_domain(&args.domain).await {
                Ok(result) => println!("{}", result),
                Err(e) => eprintln!("Error during WHOIS lookup: {}", e),
            }
        }

        // ─────────────────────────────── IP INFO LOOKUP ───────────────────────────
        Some(Commands::IpInfo(args)) => {
            if let Err(e) = ip_info_ops::lookup_ip_info(&args.ip, args.abuse, args.asn).await {
                eprintln!("Error during IP lookup: {}", e);
            }
        }
        
        // ─────────────────────────────── FILE DOWNLOAD ────────────────────────────
        Some(Commands::Download(args)) => {
            // Extract filename from URL if output is not specified
            let output_path = match args.output {
                Some(path) => path,
                None => {
                    // Extract filename from URL
                    let url_parts: Vec<&str> = args.url.split('/').collect();
                    let filename = url_parts.last()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&"downloaded_file")
                        .to_string();
                    
                    PathBuf::from(filename)
                }
            };
            
            if let Err(e) = file_download_ops::download_file(
                &args.url,
                &output_path,
                args.retries,
                args.resume,
                args.parallel
            ).await {
                eprintln!("Error during file download: {}", e);
            }
        }
        
        // ─────────────────────────────── VIDEO DOWNLOAD ────────────────────────────
        Some(Commands::VideoDownload(args)) => {
            // Either get info or download the video
            if args.info_only {
                match video_download_ops::get_video_info(&args.url).await {
                    Ok(info) => println!("{}", info),
                    Err(e) => eprintln!("Error getting video info: {}", e),
                }
            } else {
                // Parse quality
                let quality = args.quality.as_deref()
                    .and_then(|q| video_download_ops::VideoQuality::from_string(q))
                    .unwrap_or(video_download_ops::VideoQuality::Best);
                
                // Get output directory
                let output_dir = args.output_dir.unwrap_or_else(|| PathBuf::from("."));
                
                // Create options with all CLI arguments
                let options = video_download_ops::DownloadOptions {
                    quality,
                    audio_only: args.audio_only,
                    max_rate: args.rate_limit,
                    concurrent_downloads: args.concurrent,
                    cookies_file: args.cookies_file,
                    subtitles: args.subtitles,
                    force_ipv4: args.force_ipv4,
                    proxy: args.proxy,
                    retries: args.retries,
                };
                
                if let Err(e) = video_download_ops::download_video_with_options(
                    &args.url, 
                    &output_dir,
                    &options,
                ).await {
                    eprintln!("Error during video download: {}", e);
                }
            }
        }

        // ─────────────────────────────── IMAGE DOWNLOAD ────────────────────────────
        Some(Commands::ImageDownload(args)) => {
            // Setup search options from CLI args
            let mut options = image_download_ops::ImageSearchOptions::default();
            options.query = args.query;
            options.count = args.count;
            options.min_width = args.min_width;
            options.min_height = args.min_height;
            options.color = args.color;
            options.safe_search = !args.unsafe_search;
            options.concurrent_downloads = args.concurrent;
            
            // Get output directory
            let output_dir = args.output_dir.unwrap_or_else(|| PathBuf::from("./images"));
            
            // Search for images
            match image_download_ops::search_images(&options).await {
                Ok(images) => {
                    if images.is_empty() {
                        println!("{}", "No images found matching your criteria.".yellow());
                    } else {
                        println!("{} {} images found", "Found".green(), images.len());
                        
                        // Download the images
                        if let Err(e) = image_download_ops::download_images(
                            &images, 
                            &output_dir, 
                            options.concurrent_downloads
                        ).await {
                            eprintln!("Error during image download: {}", e);
                        }
                    }
                },
                Err(e) => eprintln!("Error during image search: {}", e),
            }
        }

        // ─────────────────────────────── PC SPECS ────────────────────────────
        Some(Commands::PCSpecs(args)) => {
            if let Some(output_path) = args.output {
                // Save to file
                if let Err(e) = pc_specs_ops::save_system_info_to_file(&output_path) {
                    eprintln!("Error saving system information: {}", e);
                }
            } else {
                // Display on screen
                if let Err(e) = pc_specs_ops::display_system_info() {
                    eprintln!("Error displaying system information: {}", e);
                }
            }
        }

        // ─────────────────────────────── INTERACTIVE ────────────────────────────
        None                                                => interactive::run_interactive_mode().await.map_err(|e| anyhow::anyhow!("{}", e))?,
    }

    Ok(())
}
