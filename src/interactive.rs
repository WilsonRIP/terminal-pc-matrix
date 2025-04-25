use crate::cli::{RenameArgs, SyncArgs, PortScanArgs, DnsCacheArgs, DnsAction, parse_ports, parse_header};
use crate::file_ops; // Assuming file_ops will contain the implementations
use crate::browser_ops;
use crate::utils::prompt;
use crate::network_ops;
use crate::http_ops;
use crate::dns_ops;

use colored::*;
use std::error::Error;
use std::path::PathBuf;
use std::collections::HashMap; // Needed for http headers

// Function to run the interactive menu (now async)
pub async fn run_interactive_mode() -> Result<(), Box<dyn Error>> {
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
async fn handle_list() -> Result<(), Box<dyn Error>> {
    println!("{}", "List Directory".magenta());
    let path_str = prompt("Enter the folder path to list (default: .)")?;
    let path = if path_str.is_empty() { PathBuf::from(".") } else { PathBuf::from(path_str) };
    file_ops::list_directory(&path).map_err(|e| e.into())
}

async fn handle_backup() -> Result<(), Box<dyn Error>> {
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

async fn handle_close_browsers() -> Result<(), Box<dyn Error>> {
    println!("{}", "Close Browsers".magenta());
    browser_ops::close_browsers()
}

async fn handle_organize_screenshots() -> Result<(), Box<dyn Error>> {
    println!("{}", "Organize Screenshots".magenta());
    file_ops::organize_screenshots()
}

async fn handle_analyze_disk() -> Result<(), Box<dyn Error>> {
    println!("{}", "Analyze Disk Usage".magenta());
    let path_str = prompt("Enter the path to analyze (default: .)")?;
    let path = if path_str.is_empty() { PathBuf::from(".") } else { PathBuf::from(path_str) };
    let top_str = prompt("How many top items to show? (default: 10)")?;
    let top = top_str.parse().unwrap_or(10);
    file_ops::analyze_disk(&path, top)
}

async fn handle_clean_system() -> Result<(), Box<dyn Error>> {
    println!("{}", "Identify Temporary Files".magenta());
    // println!("{}", "Running system clean identification (Dry Run)...".yellow());
    file_ops::clean_system(true) // Always dry-run for now
}

async fn handle_rename() -> Result<(), Box<dyn Error>> {
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

async fn handle_find_duplicates() -> Result<(), Box<dyn Error>> {
    println!("{}", "Find Duplicate Files".magenta());
    let path_str = prompt("Enter directory to search for duplicates (default: .)")?;
    let min_size_str = prompt("Enter minimum file size (e.g., 1k, default: 1k)")?;

    let path = if path_str.is_empty() { PathBuf::from(".") } else { PathBuf::from(path_str) };
    let min_size = if min_size_str.is_empty() { "1k".to_string() } else { min_size_str };

    file_ops::find_duplicates(&path, &min_size)
}

async fn handle_sync_folders() -> Result<(), Box<dyn Error>> {
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

async fn handle_search_files() -> Result<(), Box<dyn Error>> {
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

async fn handle_bandwidth() -> Result<(), Box<dyn Error>> {
    println!("{}", "Network Bandwidth Snapshot".magenta());
    network_ops::get_bandwidth_snapshot().await
}

async fn handle_port_scan() -> Result<(), Box<dyn Error>> {
    println!("{}", "Port Scanner".magenta());
    let host = prompt("Enter host IP or name to scan")?;
    if host.is_empty() {
        return Err("Host cannot be empty.".into());
    }
    let ports_str = prompt("Enter ports (e.g., 80, 1-1024, default: 1-1024)")?;
    let ports = parse_ports(if ports_str.is_empty() { "1-1024" } else { &ports_str })?;

    // For now, use a default timeout. Could add prompt later.
    let args = PortScanArgs { host, ports, timeout: 100 }; 

    network_ops::scan_ports(&args.host, &args.ports, args.timeout).await
}

async fn handle_http_request() -> Result<(), Box<dyn Error>> {
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

async fn handle_dns_flush() -> Result<(), Box<dyn Error>> {
    println!("{}", "Flush DNS Cache".magenta());
    let args = DnsCacheArgs { action: DnsAction::Flush };
    dns_ops::manage_dns(args.action).await
} 