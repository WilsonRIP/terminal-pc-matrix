use crate::cli::{RenameArgs, SyncArgs, PortScanArgs, DnsCacheArgs, DnsAction, parse_ports, parse_header};
use crate::file_ops; // Assuming file_ops will contain the implementations
use crate::browser_ops::{self, BrowserType, BrowserDataType};
use crate::utils::prompt;
use crate::network_ops;
use crate::http_ops;
use crate::dns_ops;
use crate::calculator_ops;
use crate::whois_ops;
use crate::ip_info_ops;
use crate::file_download_ops;
use crate::video_download_ops;
use crate::image_download_ops;
use crate::antivirus_ops;
use crate::pc_specs_ops;
use crate::audio_text_ops;

use colored::*;
use std::error::Error;
use std::path::PathBuf;
use std::collections::HashMap; // Needed for http headers
use std::io::{self}; // Remove Write
use anyhow::{anyhow, Result}; // Add anyhow macro import
use clap::{Arg, ArgAction, Command as ClapCommand};

type BoxedError = Box<dyn Error + Send + Sync>;

// Function to run the interactive menu (now async)
pub async fn start_interactive_mode() -> Result<(), BoxedError> {
    loop {
        println!("\n{}", "--- Options ---".magenta().bold());
        println!("  {} List files in a folder", "1.".cyan());
        println!("  {} Backup a folder", "2.".cyan());
        println!("  {} Close all major web browsers", "3.".cyan());
        println!("  {} [macOS only] Organize Desktop screenshots", "4.".cyan());
        println!("  {} Analyze Disk Usage", "5.".cyan());
        println!("  {} [EXPERIMENTAL] Identify Temporary Files", "6.".cyan());
        println!("  {} Batch Rename Files (Regex)", "7.".cyan());
        println!("  {} Find Duplicate Files", "8.".cyan());
        println!("  {} Sync Folders (One-Way)", "9.".cyan());
        println!("  {} Search Files by Name", "10.".cyan());
        println!("  {} Network Bandwidth Snapshot", "11.".cyan());
        println!("  {} Scan Host Ports", "12.".cyan());
        println!("  {} Make HTTP Request", "13.".cyan());
        println!("  {} Flush DNS Cache", "14.".cyan());
        println!("  {} Discover Network Devices", "15.".cyan());
        println!("  {} Ping Host", "16.".cyan());
        println!("  {} Browser Management", "17.".cyan());
        println!("  {} Calculator", "18.".cyan());
        println!("  {} WHOIS Lookup", "19.".cyan());
        println!("  {} IP/Geo/ASN Information", "20.".cyan());
        println!("  {} Download File", "21.".cyan());
        println!("  {} Video Downloader", "22.".cyan());
        println!("  {} Image Downloader", "23.".cyan());
        println!("  {} Antivirus Scanner", "24.".cyan());
        println!("  {} PC Specs", "25.".cyan());
        println!("  {} Audio Transcribe", "26.".cyan());
        println!("  {} Quit", "q.".yellow());

        let choice = prompt(&"Choose an option".bold().to_string())?;

        // Wrap handlers in blocks to manage scope and add separators
        let handler_result = match choice.as_str() {
            "1" => { handle_list().await }
            "2" => { handle_backup().await }
            "3" => { handle_close_browsers().await }
            "4" => { handle_organize_screenshots().await }
            "5" => { handle_analyze_disk().await }
            "6" => { handle_clean_system().await }
            "7" => { handle_rename().await }
            "8" => { handle_find_duplicates().await }
            "9" => { handle_sync_folders().await }
            "10" => { handle_search_files().await }
            "11" => { handle_bandwidth().await }
            "12" => { handle_port_scan().await }
            "13" => { handle_http_request().await }
            "14" => { handle_dns_flush().await }
            "15" => { handle_network_devices().await }
            "16" => { handle_ping().await }
            "17" => { handle_browser_management().await }
            "18" => { handle_calculator().await }
            "19" => { handle_whois_lookup().await }
            "20" => { handle_ip_info().await }
            "21" => { handle_file_download().await }
            "22" => { handle_video_download().await }
            "23" => { handle_image_download().await }
            "24" => { handle_antivirus().await }
            "25" => { handle_pc_specs().await }
            "26" => { handle_audio_transcribe().await.map_err(|e| format!("{}", e)) }
            "q" => {
                println!("{}", "Exiting application.".yellow());
                break; // Exit loop
            }
            _ => {
                eprintln!("{}", "Invalid choice.".red());
                Ok(()) // Continue loop on invalid choice
            }
        };

        // Print separator after handler execution (unless quitting)
        if choice != "q" {
            println!("{}", "---".dimmed());
        }

        // Handle errors from the executed handler
        if let Err(e) = handler_result {
             eprintln!("{}: {}", "Operation failed".red().bold(), e);
             // Decide whether to continue or break on error? For now, continue.
        }
    }
    Ok(())
}

// Helper functions for interactive choices
async fn handle_list() -> Result<(), BoxedError> {
    println!("{}", "List Directory".magenta());
    let path_str = prompt("Enter the folder path to list (default: .)")?;
    let path = if path_str.is_empty() { PathBuf::from(".") } else { PathBuf::from(path_str) };
    file_ops::list_directory(&path).map_err(|e| e.into())
}

async fn handle_backup() -> Result<(), BoxedError> {
    println!("{}", "Backup Directory".magenta());
    let source_str = prompt("Enter the source folder path to backup")?;
    if source_str.is_empty() {
        return Err("Source path cannot be empty.".into());
    }
    let destination_str = prompt("Enter the destination path for the backup")?;
    if destination_str.is_empty() {
        return Err("Destination path cannot be empty.".into());
    }
    let source_path = PathBuf::from(source_str);
    let destination_path = PathBuf::from(destination_str);
    file_ops::backup_directory(&source_path, &destination_path).map_err(|e| e.into())
}

async fn handle_close_browsers() -> Result<(), BoxedError> {
    println!("{}", "Close Browsers".magenta());
    browser_ops::close_browsers()
}

async fn handle_organize_screenshots() -> Result<(), BoxedError> {
    println!("{}", "Organize Screenshots".magenta());
    file_ops::organize_screenshots()
}

async fn handle_analyze_disk() -> Result<(), BoxedError> {
    println!("{}", "Analyze Disk Usage".magenta());
    let path_str = prompt("Enter directory path to analyze")?;
    let path = if path_str.is_empty() { 
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    } else {
        PathBuf::from(path_str)
    };
    let top_str = prompt("Show top N files by size (default: 10)")?;
    let top = top_str.parse().unwrap_or(10);
    file_ops::analyze_disk(&path, top)
}

async fn handle_clean_system() -> Result<(), BoxedError> {
    println!("{}", "Clean System Cache/Temporary Files".magenta());
    let msg = "This is an EXPERIMENTAL feature that will show temporary and cache files."; 
    println!("{} {}", "⚠️".yellow(), msg.yellow());
    file_ops::clean_system(true) // Always dry-run for now
}

async fn handle_rename() -> Result<(), BoxedError> {
    println!("{}", "Batch Rename Files".magenta());
    let dir_str = prompt("Enter directory containing files to rename (default: .)")?;
    let pattern_str = prompt("Enter regex pattern to match filenames")?;
    if pattern_str.is_empty() {
        return Err("Pattern cannot be empty.".into());
    }
    let replacement_str = prompt("Enter replacement string (use $1, $2 for captures)")?;
    let dry_run_str = prompt("Perform dry run? (yes/no, default: yes)")?;

    let dir = if dir_str.is_empty() { PathBuf::from(".") } else { PathBuf::from(dir_str) };
    let dry_run = !dry_run_str.trim().eq_ignore_ascii_case("no");

    let args = RenameArgs {
        directory: dir,
        pattern: pattern_str,
        replacement: replacement_str,
        dry_run,
    };

    file_ops::rename_files(&args)
}

async fn handle_find_duplicates() -> Result<(), BoxedError> {
    println!("{}", "Find Duplicate Files".magenta());
    let path_str = prompt("Enter directory to search for duplicates (default: .)")?;
    let min_size_str = prompt("Enter minimum file size (e.g., 1k, default: 1k)")?;

    let path = if path_str.is_empty() { PathBuf::from(".") } else { PathBuf::from(path_str) };
    let min_size = if min_size_str.is_empty() { "1k".to_string() } else { min_size_str };

    file_ops::find_duplicates(&path, &min_size)
}

async fn handle_sync_folders() -> Result<(), BoxedError> {
    println!("{}", "Sync Folders (One-Way)".magenta());
    let source_str = prompt("Enter the source directory")?;
    if source_str.is_empty() {
        return Err("Source path cannot be empty.".into());
    }
    let dest_str = prompt("Enter the destination directory")?;
    if dest_str.is_empty() {
        return Err("Destination path cannot be empty.".into());
    }
    let delete_str = prompt("Delete extra files in destination? (yes/no, default: no)")?;
    let dry_run_str = prompt("Perform dry run? (yes/no, default: yes)")?;

    let sync_args = SyncArgs {
        source: PathBuf::from(source_str),
        destination: PathBuf::from(dest_str),
        dry_run: !dry_run_str.trim().eq_ignore_ascii_case("no"),
        delete: delete_str.trim().eq_ignore_ascii_case("yes"),
    };

     file_ops::sync_folders(&sync_args)
}

async fn handle_search_files() -> Result<(), BoxedError> {
    println!("{}", "Search Files".magenta());
    let path_str = prompt("Enter directory to search within (default: .)")?;
    let query_str = prompt("Enter filename pattern to search for")?;
    if query_str.is_empty() {
        return Err("Search query cannot be empty.".into());
    }

    let path = if path_str.is_empty() { PathBuf::from(".") } else { PathBuf::from(path_str) };

    file_ops::search_files(&path, &query_str)
}

// --- New Handler Functions (async) ---

async fn handle_bandwidth() -> Result<(), BoxedError> {
    println!("{}", "Network Bandwidth Snapshot".magenta());
    network_ops::get_bandwidth_snapshot().await.map_err(|e| anyhow!("{}", e).into())
}

async fn handle_port_scan() -> Result<(), BoxedError> {
    println!("{}", "Port Scanner".magenta());
    let host = prompt("Enter host IP or name to scan")?;
    if host.is_empty() {
        return Err("Host cannot be empty.".into());
    }
    let ports_str = prompt("Enter ports (e.g., 80, 1-1024, default: 1-1024)")?;
    let ports = parse_ports(if ports_str.is_empty() { "1-1024" } else { &ports_str })?;

    // For now, use a default timeout. Could add prompt later.
    let args = PortScanArgs { host, ports, timeout: 100 }; 

    network_ops::scan_ports(&args.host, &args.ports, args.timeout).await.map_err(|e| anyhow!("{}", e).into())
}

async fn handle_http_request() -> Result<(), BoxedError> {
    println!("{}", "HTTP Request Tool".magenta());
    let url = prompt("Enter URL")?;
    if url.is_empty() {
        return Err("URL cannot be empty.".into());
    }
    let method_str = prompt("Enter HTTP method (default: GET)")?;
    let method = if method_str.is_empty() { "GET".to_string() } else { method_str.to_uppercase() };

    let mut body: Option<String> = None;
    if method == "POST" || method == "PUT" || method == "PATCH" {
        body = Some(prompt(&format!("Enter request body for {}", method))?);
    }

    let mut headers_map: HashMap<String, String> = HashMap::new();
    loop {
        let header_str = prompt("Add header (key=value) or press Enter to continue")?;
        if header_str.is_empty() {
            break;
        }
        match parse_header(&header_str) {
            Ok((key, value)) => {
                headers_map.insert(key, value);
            }
            Err(e) => eprintln!("{}: {}", "Invalid header format".yellow(), e),
        }
    }

    // Convert HashMap to Vec<(String, String)> if needed by http_ops::make_request
    // Or adjust make_request to accept HashMap

    http_ops::make_request(&method, &url, body.as_deref(), &headers_map).await

}

async fn handle_dns_flush() -> Result<(), BoxedError> {
    println!("{}", "Flush DNS Cache".magenta());
    let args = DnsCacheArgs { action: DnsAction::Flush };
    dns_ops::manage_dns(args.action).await
}

// New handler for network device discovery
async fn handle_network_devices() -> Result<(), BoxedError> {
    println!("{}", "Network Device Discovery".magenta());
    let timeout_str = prompt("Enter scan timeout in ms (default: 100)")?;
    let timeout = timeout_str.parse().unwrap_or(100);
    
    network_ops::discover_network_devices(timeout).await.map_err(|e| anyhow!("{}", e).into())
} 

// Handler for ping functionality
async fn handle_ping() -> Result<(), BoxedError> {
    println!("{}", "Ping Host".magenta());
    let host = prompt("Enter hostname or IP address to ping")?;
    if host.is_empty() {
        return Err("Host cannot be empty.".into());
    }
    
    let count_str = prompt("Number of ping packets to send (default: 4)")?;
    let count = count_str.parse().unwrap_or(4);
    
    network_ops::ping_host(&host, count).await.map_err(|e| anyhow!("{}", e).into())
}

// Handler for Browser Management
async fn handle_browser_management() -> Result<(), BoxedError> {
    println!("{}", "Browser Management".magenta());

    // Choose browser
    println!("Select browser:");
    let browsers = [
        (BrowserType::Chrome, "Chrome"),
        (BrowserType::Firefox, "Firefox"),
        (BrowserType::Edge, "Edge"),
        (BrowserType::Brave, "Brave"),
        (BrowserType::Safari, "Safari (macOS only)"),
        (BrowserType::Opera, "Opera"),
        (BrowserType::Vivaldi, "Vivaldi"),
        // Add other supported browsers here
    ];
    for (i, (_, name)) in browsers.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    let browser_choice_str = prompt("Enter browser number")?;
    let browser_idx: usize = browser_choice_str.parse().map_err(|_| "Invalid number")?;
    if browser_idx == 0 || browser_idx > browsers.len() {
        return Err("Invalid browser selection.".into());
    }
    let (selected_browser, browser_name) = browsers[browser_idx - 1].clone();

    // Choose operation
    println!("Select operation for {}:", browser_name.cyan());
    let operations = [
        (BrowserDataType::History, "Delete History"),
        (BrowserDataType::Cookies, "Delete Cookies"),
        (BrowserDataType::Bookmarks, "Export Bookmarks"),
        (BrowserDataType::Passwords, "Export Passwords (experimental; Safari not supported)"),
    ];
    for (i, (_, name)) in operations.iter().enumerate() {
        // Disable password export for Safari explicitly
        if selected_browser == BrowserType::Safari && operations[i].0 == BrowserDataType::Passwords {
             println!("  {}. {} {}", i + 1, name, "(Not Supported)".dimmed());
        } else {
            println!("  {}. {}", i + 1, name);
        }
    }
    let op_choice_str = prompt("Enter operation number")?;
    let op_idx: usize = op_choice_str.parse().map_err(|_| "Invalid number")?;
    if op_idx == 0 || op_idx > operations.len() {
        return Err("Invalid operation selection.".into());
    }
    let (selected_operation, _) = operations[op_idx - 1].clone();

    // Prevent Safari password export attempt
    if selected_browser == BrowserType::Safari && selected_operation == BrowserDataType::Passwords {
        return Err("Password export is not supported for Safari.".into());
    }

    // Execute operation
    println!("Performing {:?} on {:?}...", selected_operation, selected_browser);
    match selected_operation {
        BrowserDataType::History | BrowserDataType::Cookies => {
            match browser_ops::delete_browser_data(selected_browser, selected_operation) {
                Ok(result) => {
                    if result.success {
                        println!("{}", result.message.green());
                    } else {
                        // This case shouldn't happen if Ok is returned, but handle defensively
                        eprintln!("{}: {}", "Operation reported non-success but no error".yellow(), result.message);
                    }
                },
                Err(e) => eprintln!("{}: {}", "Error deleting data".red(), e),
            }
        }
        BrowserDataType::Bookmarks | BrowserDataType::Passwords => {
            match browser_ops::export_browser_data(selected_browser, selected_operation) {
                Ok(result) => {
                    if result.success {
                        println!("{}", result.message.green());
                    } else {
                        // This case shouldn't happen if Ok is returned, but handle defensively
                        eprintln!("{}: {}", "Operation reported non-success but no error".yellow(), result.message);
                    }
                },
                Err(e) => eprintln!("{}: {}", "Error exporting data".red(), e),
            }
        }
    }

    Ok(())
}

// Handler for Calculator
async fn handle_calculator() -> Result<(), BoxedError> {
    println!("{}", "Simple Calculator (Type 'q' to exit)".magenta());
    loop {
        let expr = prompt(">>")?;
        if expr.eq_ignore_ascii_case("q") {
            break;
        }
        if expr.is_empty() {
            continue;
        }

        // evaluation happens synchronously within the async handler
        match calculator_ops::evaluate_expression(&expr) {
            Ok(_) => { /* Result already printed by evaluate_expression */ }
            Err(e) => eprintln!("{}: {}", "Calculation Error".red(), e),
        }
    }
    Ok(())
}

// Handler for WHOIS Lookup
async fn handle_whois_lookup() -> Result<(), BoxedError> {
    println!("{}", "WHOIS Lookup".magenta());
    let domain = prompt("Enter domain name to lookup (e.g., google.com)")?;
    if domain.is_empty() {
        return Err("Domain name cannot be empty.".into());
    }
    
    match whois_ops::lookup_domain(&domain).await {
        Ok(result) => {
            println!("{}", result);
            Ok(())
        },
        Err(e) => Err(anyhow!("WHOIS lookup failed: {}", e).into())
    }
}

// Handler for IP Information Lookup
async fn handle_ip_info() -> Result<(), BoxedError> {
    println!("{}", "IP/Geo/ASN Information Lookup".magenta());
    let ip = prompt("Enter IP address to lookup (e.g., 8.8.8.8)")?;
    if ip.is_empty() {
        return Err("IP address cannot be empty.".into());
    }
    
    let show_abuse_str = prompt("Include abuse contact information? (yes/no, default: no)")?;
    let show_abuse = show_abuse_str.trim().eq_ignore_ascii_case("yes");
    
    let show_asn_str = prompt("Show ASN information? (yes/no, default: no)")?;
    let show_asn = show_asn_str.trim().eq_ignore_ascii_case("yes");
    
    ip_info_ops::lookup_ip_info(&ip, show_abuse, show_asn).await.map_err(|e| anyhow!("IP info lookup failed: {}", e).into())
}

// Handler for File Download
async fn handle_file_download() -> Result<(), BoxedError> {
    println!("{}", "HTTP File Downloader".magenta());
    
    let url = prompt("Enter URL of the file to download")?;
    if url.is_empty() {
        return Err("URL cannot be empty.".into());
    }
    
    // Extract filename from URL if output is not specified
    let default_filename = {
        let url_parts: Vec<&str> = url.split('/').collect();
        url_parts.last()
            .filter(|s| !s.is_empty())
            .unwrap_or(&"downloaded_file")
            .to_string()
    };
    
    let output_str = prompt(&format!("Enter output path (default: {})", default_filename))?;
    let output_path = if output_str.is_empty() {
        PathBuf::from(default_filename)
    } else {
        PathBuf::from(output_str)
    };
    
    let retries_str = prompt("Number of retries (default: 5)")?;
    let retries = retries_str.parse().unwrap_or(5);
    
    let resume_str = prompt("Resume download if file exists? (yes/no, default: yes)")?;
    let resume = !resume_str.trim().eq_ignore_ascii_case("no");
    
    let parallel_str = prompt("Number of parallel connections (default: 1)")?;
    let parallel = parallel_str.parse().unwrap_or(1);
    
    file_download_ops::download_file(&url, &output_path, retries, resume, parallel).await.map_err(|e| anyhow!("Download failed: {}", e).into())
}

// Handler for Video Download
async fn handle_video_download() -> Result<(), BoxedError> {
    println!("{}", "Video Downloader".magenta());
    
    // Check if yt-dlp is installed
    if !video_download_ops::check_ytdlp_installed().await {
        println!("{}", "yt-dlp is not installed. Please install it first:".red());
        println!("    {}", "https://github.com/yt-dlp/yt-dlp#installation".yellow());
        return Err("yt-dlp not installed".into());
    }
    
    // Get video URL
    let url = prompt("Enter URL of the video to download")?;
    if url.is_empty() {
        return Err("URL cannot be empty.".into());
    }
    
    // Ask if the user wants to download or just get info
    let info_mode_str = prompt("Just display video info? (yes/no, default: no)")?;
    let info_mode = info_mode_str.trim().eq_ignore_ascii_case("yes");
    
    if info_mode {
        // Show video information
        match video_download_ops::get_video_info(&url).await {
            Ok(info) => {
                println!("\n{}", "Video Information:".cyan().bold());
                println!("{}", info);
            },
            Err(e) => return Err(anyhow!("Failed to get video info: {}", e).into()),
        }
        return Ok(());
    }
    
    // Get output directory
    let output_dir_str = prompt("Enter output directory (default: current directory)")?;
    let output_dir = if output_dir_str.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(output_dir_str)
    };
    
    // Create download options struct with defaults
    let mut options = video_download_ops::DownloadOptions::default();
    
    // Get quality preference
    println!("\n{}", "Quality Options:".cyan());
    println!("  1. Best quality");
    println!("  2. 1080p HD");
    println!("  3. 720p HD");
    println!("  4. 480p SD");
    println!("  5. Lowest quality (saves bandwidth)");
    println!("  6. Audio only (MP3)");
    
    let quality_choice = prompt("Select quality (1-6, default: 1)")?;
    options.quality = match quality_choice.as_str() {
        "2" => video_download_ops::VideoQuality::HD1080,
        "3" => video_download_ops::VideoQuality::HD720,
        "4" => video_download_ops::VideoQuality::SD480,
        "5" => video_download_ops::VideoQuality::Lowest,
        "6" => video_download_ops::VideoQuality::AudioOnly,
        _ => video_download_ops::VideoQuality::Best,
    };
    
    // Check if audio only is selected
    options.audio_only = options.quality == video_download_ops::VideoQuality::AudioOnly;
    
    // If not audio only, ask if they want to extract audio
    if !options.audio_only {
        let extract_audio_str = prompt("Extract audio only? (yes/no, default: no)")?;
        options.audio_only = extract_audio_str.trim().eq_ignore_ascii_case("yes");
    }
    
    // Ask about performance optimizations
    println!("\n{}", "Performance Options:".cyan());
    
    // Ask about parallel downloads for playlists
    let parallel_str = prompt("Number of parallel downloads for playlists (1-10, default: 3)")?;
    if !parallel_str.is_empty() {
        if let Ok(parallel) = parallel_str.parse::<usize>() {
            if parallel > 0 && parallel <= 10 {
                options.concurrent_downloads = parallel;
            }
        }
    }
    
    // Ask about rate limiting
    let rate_limit_str = prompt("Rate limit in bytes/s? (e.g., 2M for 2MB/s, leave empty for unlimited)")?;
    if !rate_limit_str.is_empty() {
        options.max_rate = Some(rate_limit_str);
    }
    
    // Ask about subtitles
    let subtitles_str = prompt("Download subtitles if available? (yes/no, default: no)")?;
    options.subtitles = subtitles_str.trim().eq_ignore_ascii_case("yes");
    
    // Ask about proxy
    let proxy_str = prompt("Use proxy? (URL or leave empty for none)")?;
    if !proxy_str.is_empty() {
        options.proxy = Some(proxy_str);
    }
    
    // Ask about retries
    let retries_str = prompt("Number of retries on failure (default: 10)")?;
    if !retries_str.is_empty() {
        if let Ok(retries) = retries_str.parse::<usize>() {
            options.retries = retries;
        }
    }
    
    // Show a summary of the download options
    println!("\n{}", "Download Summary:".cyan().bold());
    println!("URL: {}", url);
    println!("Output directory: {}", output_dir.display());
    println!("Quality: {:?}", options.quality);
    println!("Audio only: {}", if options.audio_only { "Yes" } else { "No" });
    if let Some(rate) = &options.max_rate {
        println!("Rate limit: {}", rate);
    }
    println!("Parallel downloads: {}", options.concurrent_downloads);
    println!("Download subtitles: {}", if options.subtitles { "Yes" } else { "No" });
    if let Some(proxy) = &options.proxy {
        println!("Using proxy: {}", proxy);
    }
    println!("Retries: {}", options.retries);
    
    let confirm_str = prompt("\nStart download with these settings? (yes/no, default: yes)")?;
    if confirm_str.trim().eq_ignore_ascii_case("no") {
        return Ok(());
    }
    
    // Perform the download with full options
    match video_download_ops::download_video_with_options(&url, &output_dir, &options).await {
        Ok(_) => {
            println!("{}", "Video downloaded successfully.".green());
            Ok(())
        },
        Err(e) => Err(anyhow!("Video download failed: {}", e).into()),
    }
}

// Handler for Image Download
async fn handle_image_download() -> Result<(), BoxedError> {
    println!("{}", "Image Downloader".magenta());
    
    // Get search query
    let query = prompt("Enter search term for images")?;
    if query.is_empty() {
        return Err("Search query cannot be empty.".into());
    }
    
    // Get number of images to download
    let count_str = prompt("Number of images to download (default: 10)")?;
    let count = count_str.parse().unwrap_or(10);
    
    // Create search options with defaults
    let mut options = image_download_ops::ImageSearchOptions::default();
    options.query = query;
    options.count = count;
    
    // Ask about filtering options
    println!("\n{}", "Filtering Options:".cyan());
    
    // Minimum dimensions
    let min_dimensions_str = prompt("Minimum dimensions (format: WIDTHxHEIGHT, default: 800x600)")?;
    if !min_dimensions_str.is_empty() {
        if let Some((width_str, height_str)) = min_dimensions_str.split_once('x') {
            if let Ok(width) = width_str.parse::<u32>() {
                options.min_width = Some(width);
            }
            if let Ok(height) = height_str.parse::<u32>() {
                options.min_height = Some(height);
            }
        }
    }
    
    // Safe search
    let safe_search_str = prompt("Enable safe search? (yes/no, default: yes)")?;
    options.safe_search = !safe_search_str.trim().eq_ignore_ascii_case("no");
    
    // Color filter
    let color_str = prompt("Filter by color? (red, green, blue, yellow, black, white, or leave empty)")?;
    if !color_str.is_empty() {
        options.color = Some(color_str);
    }
    
    // Concurrent downloads
    let concurrent_str = prompt("Number of concurrent downloads (1-10, default: 5)")?;
    if !concurrent_str.is_empty() {
        if let Ok(concurrent) = concurrent_str.parse::<usize>() {
            if concurrent > 0 && concurrent <= 10 {
                options.concurrent_downloads = concurrent;
            }
        }
    }
    
    // Get output directory
    let output_dir_str = prompt("Enter output directory (default: ./images)")?;
    let output_dir = if output_dir_str.is_empty() {
        PathBuf::from("./images")
    } else {
        PathBuf::from(output_dir_str)
    };
    
    // Show a summary
    println!("\n{}", "Search Summary:".cyan().bold());
    println!("Query: {}", options.query);
    println!("Count: {}", options.count);
    println!("Safe search: {}", if options.safe_search { "Yes" } else { "No" });
    if let Some(color) = &options.color {
        println!("Color filter: {}", color);
    }
    println!("Min dimensions: {}x{}", 
        options.min_width.unwrap_or(0), 
        options.min_height.unwrap_or(0));
    println!("Output directory: {}", output_dir.display());
    println!("Concurrent downloads: {}", options.concurrent_downloads);
    
    // Confirm
    let confirm_str = prompt("\nSearch for images with these settings? (yes/no, default: yes)")?;
    if confirm_str.trim().eq_ignore_ascii_case("no") {
        return Ok(());
    }
    
    // Search for images
    match image_download_ops::search_images(&options).await {
        Ok(images) => {
            if images.is_empty() {
                println!("{}", "No images found matching your criteria.".yellow());
                return Ok(());
            }
            
            println!("\n{} {} images found. Preview sample:", "Success!".green(), images.len());
            
            // Show preview of first few images
            let preview_count = std::cmp::min(3, images.len());
            for i in 0..preview_count {
                println!("\n{} {}:", "Image".cyan().bold(), i+1);
                image_download_ops::display_image_info(&images[i]);
            }
            
            if images.len() > preview_count {
                println!("\n{} more images found...", images.len() - preview_count);
            }
            
            // Ask to download
            let download_str = prompt("\nDownload these images? (yes/no, default: yes)")?;
            if download_str.trim().eq_ignore_ascii_case("no") {
                return Ok(());
            }
            
            // Download images
            match image_download_ops::download_images(&images, &output_dir, options.concurrent_downloads).await {
                Ok(_) => {
                    println!("\n{}", "Images downloaded successfully.".green());
                    Ok(())
                },
                Err(e) => Err(anyhow!("Image download failed: {}", e).into()),
            }
        },
        Err(e) => Err(anyhow!("Image search failed: {}", e).into()),
    }
}

// Handler for Antivirus Scan
async fn handle_antivirus() -> Result<(), BoxedError> {
    println!("{}", "Antivirus Scanner".magenta());
    
    // First check if ClamAV is installed
    if !antivirus_ops::check_clamav_installed() {
        return Err("ClamAV is not installed. Please install ClamAV to use this feature.".into());
    }
    
    // Get ClamAV version
    match antivirus_ops::get_clamav_info() {
        Ok(version) => println!("{} {}", "ClamAV Version:".green(), version),
        Err(e) => println!("{} {}", "Couldn't get ClamAV version:".yellow(), e),
    }
    
    // Show scan options
    println!("\n{}", "Select scan type:".cyan());
    println!("  1. Scan single file");
    println!("  2. Scan directory (non-recursive)");
    println!("  3. Scan directory recursively");
    println!("  4. Update virus definitions");
    
    let scan_type = prompt("Enter option")?;
    
    match scan_type.as_str() {
        "1" => {
            // Scan a single file
            let file_path = prompt("Enter file path to scan")?;
            if file_path.is_empty() {
                return Err("File path cannot be empty.".into());
            }
            
            let path = PathBuf::from(file_path);
            match antivirus_ops::scan_file(&path) {
                Ok(result) => {
                    match result.status {
                        antivirus_ops::ScanStatus::Clean => {
                            println!("{} {}", "✅ Clean:".green(), path.display());
                        },
                        antivirus_ops::ScanStatus::Infected => {
                            println!("{} {} - {}", "🔴 Infected:".red(), path.display(), 
                                     result.threat_name.unwrap_or_else(|| "Unknown threat".to_string()));
                            
                            // Ask if the user wants to quarantine the file
                            let quarantine = prompt("Quarantine this file? (yes/no, default: no)")?;
                            if quarantine.trim().eq_ignore_ascii_case("yes") {
                                let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                                let quarantine_dir = home_dir.join(".quarantine");
                                
                                match antivirus_ops::quarantine_file(&path, &quarantine_dir) {
                                    Ok(new_path) => println!("{} {} -> {}", "File quarantined:".green(), path.display(), new_path.display()),
                                    Err(e) => println!("{} {}", "Failed to quarantine file:".red(), e),
                                }
                            }
                        },
                        antivirus_ops::ScanStatus::Error => {
                            println!("{} {} - {}", "❌ Error:".yellow(), path.display(), 
                                     result.threat_name.unwrap_or_else(|| "Unknown error".to_string()));
                        },
                        antivirus_ops::ScanStatus::Skipped => {
                            println!("{} {}", "⏭️ Skipped:".blue(), path.display());
                        },
                    }
                },
                Err(e) => println!("{} {}", "Scan failed:".red(), e),
            }
        },
        "2" | "3" => {
            // Scan a directory
            let dir_path = prompt("Enter directory path to scan")?;
            let path = if dir_path.is_empty() { 
                dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
            } else {
                PathBuf::from(dir_path)
            };
            
            let recursive = scan_type == "3";
            println!("{} {} ({})", "Scanning directory:".cyan(), path.display(), 
                     if recursive { "recursive" } else { "non-recursive" });
            
            match antivirus_ops::scan_directory(&path, recursive) {
                Ok(results) => {
                    // Print scan results
                    let formatted_results = antivirus_ops::format_scan_results(&results);
                    println!("{}", formatted_results);
                    
                    // If we found infected files, offer to quarantine them
                    let infected_files: Vec<_> = results.iter()
                        .filter(|r| r.status == antivirus_ops::ScanStatus::Infected)
                        .collect();
                    
                    if !infected_files.is_empty() {
                        let quarantine = prompt("Quarantine infected files? (yes/no, default: no)")?;
                        if quarantine.trim().eq_ignore_ascii_case("yes") {
                            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                            let quarantine_dir = home_dir.join(".quarantine");
                            
                            for result in infected_files {
                                match antivirus_ops::quarantine_file(&result.path, &quarantine_dir) {
                                    Ok(new_path) => println!("{} {} -> {}", "File quarantined:".green(), result.path.display(), new_path.display()),
                                    Err(e) => println!("{} {}: {}", "Failed to quarantine file".red(), result.path.display(), e),
                                }
                            }
                        }
                    }
                },
                Err(e) => println!("{} {}", "Scan failed:".red(), e),
            }
        },
        "4" => {
            // Update virus definitions
            println!("{}", "Updating virus definitions...".cyan());
            match antivirus_ops::update_virus_definitions() {
                Ok(update_info) => {
                    println!("{} {}", "Update successful:".green(), update_info);
                },
                Err(e) => {
                    println!("{} {}", "Update failed:".red(), e);
                },
            }
        },
        _ => {
            return Err("Invalid option.".into());
        }
    }
    
    Ok(())
}

// Handler for PC Specs
async fn handle_pc_specs() -> Result<(), BoxedError> {
    println!("{}", "PC Specifications".magenta());
    
    println!("\n{}", "Select an option:".cyan());
    println!("  1. View PC specifications");
    println!("  2. Save PC specifications to file");
    
    let option = prompt("Enter option")?;
    
    match option.as_str() {
        "1" => {
            pc_specs_ops::display_system_info().map_err(|e| anyhow!("{}", e).into())
        },
        "2" => {
            let file_path = prompt("Enter file path to save PC specs (default: pc_specs.txt)")?;
            let path = if file_path.is_empty() {
                PathBuf::from("pc_specs.txt")
            } else {
                PathBuf::from(file_path)
            };
            
            pc_specs_ops::save_system_info_to_file(&path).map_err(|e| anyhow!("{}", e).into())
        },
        _ => {
            Err("Invalid option.".into())
        }
    }
}

// Add this function to handle the audio transcribe menu option
async fn handle_audio_transcribe() -> Result<(), String> {
    println!("{}", "===== Audio Transcription =====".magenta().bold());
    
    // Get file path
    println!("Enter the path to the audio or video file:");
    let file_path = read_line().map_err(|e| format!("Failed to read input: {}", e))?;
    if file_path.trim().is_empty() {
        return Err("File path cannot be empty".to_string());
    }
    
    // Model size
    println!("Select model size (default: base):");
    println!("1. Tiny (fastest, least accurate)");
    println!("2. Base (default)");
    println!("3. Small");
    println!("4. Medium");
    println!("5. Large (slowest, most accurate)");
    let model_choice = read_line().map_err(|e| format!("Failed to read input: {}", e))?;
    
    let model_size = match model_choice.trim() {
        "1" => audio_text_ops::ModelSize::Tiny,
        "2" | "" => audio_text_ops::ModelSize::Base,
        "3" => audio_text_ops::ModelSize::Small,
        "4" => audio_text_ops::ModelSize::Medium,
        "5" => audio_text_ops::ModelSize::Large,
        _ => audio_text_ops::ModelSize::Base,
    };
    
    // Output path
    println!("Enter output file path (leave empty for default):");
    let output_path_str = read_line().map_err(|e| format!("Failed to read input: {}", e))?;
    let output_path = if output_path_str.trim().is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(output_path_str.trim()))
    };
    
    // Output formats
    println!("Generate SRT subtitle file? (Y/n):");
    let srt_choice = read_line().map_err(|e| format!("Failed to read input: {}", e))?;
    let srt_output = !srt_choice.trim().to_lowercase().starts_with('n');
    
    println!("Generate TXT transcript file? (Y/n):");
    let txt_choice = read_line().map_err(|e| format!("Failed to read input: {}", e))?;
    let txt_output = !txt_choice.trim().to_lowercase().starts_with('n');
    
    println!("Include timestamps in transcript? (Y/n):");
    let timestamps_choice = read_line().map_err(|e| format!("Failed to read input: {}", e))?;
    let include_timestamps = !timestamps_choice.trim().to_lowercase().starts_with('n');
    
    // Create options
    let options = audio_text_ops::TranscriptionOptions {
        model_size,
        output_file: output_path,
        save_timestamps: include_timestamps,
        output_srt: srt_output,
        output_txt: txt_output,
    };
    
    // Perform transcription
    let input_path = std::path::PathBuf::from(file_path.trim());
    
    println!("{}", "Starting transcription process...".cyan());
    match audio_text_ops::handle_audio_transcription(&input_path, options).await {
        Ok(transcript) => {
            println!("{}", "Transcription completed successfully.".green());
            Ok(())
        },
        Err(e) => {
            println!("{} {}", "Error:".red(), e);
            Err(format!("Transcription failed: {}", e))
        }
    }
}

// Helper function to read a line from stdin
fn read_line() -> io::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer.trim().to_string())
}