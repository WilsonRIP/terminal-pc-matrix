use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>, // Make the command optional for interactive mode
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List files and directories in a specified path
    List {
        /// The path to list contents of (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Backup a directory to another location
    Backup {
        /// The source directory to backup
        source: PathBuf,
        /// The destination directory for the backup
        destination: PathBuf,
    },
    /// Close all major web browsers
    CloseBrowsers,
    /// [macOS only] Organize screenshots on the Desktop into a 'Screenshots' folder
    OrganizeScreenshots,
    /// Analyze disk usage for a given path, showing large files
    AnalyzeDisk {
        /// The path to analyze (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Number of largest files/directories to show
        #[arg(short, long, default_value_t = 10)]
        top: usize,
    },
    /// [EXPERIMENTAL] Identify temporary files and cache locations
    CleanSystem {
        /// Show what would be identified without actually deleting
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        // TODO: Add --delete flag later with confirmation
    },
    /// Batch rename files in a directory using regex
    Rename(RenameArgs),
    /// Find duplicate files in a directory based on content hash
    FindDuplicates {
        /// The path to search for duplicates (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Minimum file size to consider for duplicates (e.g., 1k, 1M)
        #[arg(short, long, default_value = "1k")]
        min_size: String,
    },
    /// Synchronize contents from a source directory to a destination (one-way)
    SyncFolders(SyncArgs),
    /// Search for files by name within a directory
    SearchFiles {
         /// The directory to search within (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// The filename pattern to search for (case-insensitive)
        query: String,
    },
    /// Show a snapshot of network bandwidth usage
    Bandwidth {},
    /// Scan a host for open TCP ports
    PortScan(PortScanArgs),
    /// Make a simple HTTP request
    HttpRequest(HttpRequestArgs),
    /// Manage local DNS cache
    DnsCache(DnsCacheArgs),
    /// Ping a host to check connectivity and response time
    Ping(PingArgs),
}

#[derive(Args, Debug)]
pub struct RenameArgs {
    /// The target directory containing files to rename
    #[arg(short, long, default_value = ".")]
    pub directory: PathBuf,
    /// The regex pattern to match filenames
    #[arg(short, long)]
    pub pattern: String,
    /// The replacement string (can use capture groups like $1, $2)
    #[arg(short, long)]
    pub replacement: String,
    /// Perform a dry run without actually renaming files
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    /// The source directory
    pub source: PathBuf,
    /// The destination directory
    pub destination: PathBuf,
    /// Perform a dry run without actually copying or deleting
    #[arg(long)]
    pub dry_run: bool,
    /// Delete files in the destination that are not present in the source
    #[arg(long)]
    pub delete: bool,
}

#[derive(Args, Debug)]
pub struct PortScanArgs {
    /// The target host (IP address or hostname) to scan
    pub host: String,
    /// Ports to scan (e.g., 80, 1-1024, 80,443,1000-2000)
    #[arg(short, long, value_parser = parse_ports, default_value = "1-1024")]
    pub ports: Vec<u16>,
    /// Timeout for each port connection in milliseconds
    #[arg(short, long, default_value_t = 100)]
    pub timeout: u64,
}

#[derive(Args, Debug)]
pub struct HttpRequestArgs {
    /// HTTP method
    #[arg(short, long, value_parser = clap::value_parser!(String), default_value = "GET")]
    pub method: String, // Use String, reqwest::Method isn't easily parsable by clap
    /// Target URL
    pub url: String,
    /// Request body (for POST, PUT, etc.)
    #[arg(short, long)]
    pub body: Option<String>,
    /// Custom headers (format: key=value)
    #[arg(short = 'H', long, value_parser = parse_header)]
    pub headers: Vec<(String, String)>,
}

#[derive(Args, Debug)]
pub struct DnsCacheArgs {
    /// Action to perform on the DNS cache
    #[arg(value_enum, default_value_t=DnsAction::Flush)]
    pub action: DnsAction,
}

#[derive(Args, Debug)]
pub struct PingArgs {
    /// The target host to ping (hostname or IP address)
    pub host: String,
    /// Number of ping packets to send
    #[arg(short, long, default_value_t = 4)]
    pub count: u32,
}

#[derive(ValueEnum, Clone, Debug, Copy)] // Add Copy
pub enum DnsAction {
    /// Flush the operating system's DNS cache
    Flush,
    // View (Difficult to implement reliably cross-platform, maybe add later)
    // View,
}

// --- Parsers for Clap --- 

/// Parses a custom header argument (key=value)
pub fn parse_header(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .ok_or_else(|| format!("Invalid header format: '{}'. Use key=value.", s))
}

/// Parses a port range string (e.g., "80", "1-1024", "80,443,1000-2000") into a Vec<u16>
pub fn parse_ports(port_str: &str) -> Result<Vec<u16>, String> {
    let mut ports = Vec::new();
    for part in port_str.split(',') {
        let part = part.trim();
        if part.contains('-') {
            if let Some((start_str, end_str)) = part.split_once('-') {
                let start: u16 = start_str.trim().parse().map_err(|_| format!("Invalid start port: {}", start_str))?;
                let end: u16 = end_str.trim().parse().map_err(|_| format!("Invalid end port: {}", end_str))?;
                if start == 0 || end == 0 {
                    return Err("Port number cannot be 0".to_string());
                }
                if start > end {
                    return Err(format!("Start port {} cannot be greater than end port {}", start, end));
                }
                for port in start..=end {
                    ports.push(port);
                }
            } else {
                return Err(format!("Invalid port range format: {}", part));
            }
        } else {
            let port: u16 = part.parse().map_err(|_| format!("Invalid port number: {}", part))?;
             if port == 0 {
                return Err("Port number cannot be 0".to_string());
            }
            ports.push(port);
        }
    }
    if ports.is_empty() {
        return Err("No ports specified".to_string());
    }
    ports.sort_unstable();
    ports.dedup();
    Ok(ports)
} 