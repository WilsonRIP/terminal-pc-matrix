use sysinfo::{System};
use colored::*;
use std::error::Error;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use humansize::{format_size, BINARY}; // Use BINARY for network speeds (KiB, MiB)

// --- Bandwidth Snapshot ---

pub async fn get_bandwidth_snapshot() -> Result<(), Box<dyn Error>> {
    println!("{}", "Fetching current network usage snapshot...".dimmed());

    let sys = System::new();

    let networks = sys.networks();

    if networks.is_empty() {
        println!("{}", "No network interfaces found.".yellow());
        return Ok(());
    }

    println!("{:<20} {:>15} {:>15}", "Interface".bold(), "Received".bold(), "Transmitted".bold());
    println!("{}", "-".repeat(52).dimmed());

    for (interface_name, data) in networks {
        println!(
            "{:<20} {:>15} {:>15}",
            interface_name.cyan(),
            format!("{}/s", format_size(data.received(), BINARY)).green(),
            format!("{}/s", format_size(data.transmitted(), BINARY)).blue()
        );
    }

    Ok(())
}

// --- Port Scanner ---

pub async fn scan_ports(host: &str, ports: &[u16], timeout_ms: u64) -> Result<(), Box<dyn Error>> {
    println!(
        "{} Scanning host '{}' for ports [{}-{}] (Timeout: {}ms)...",
        "Running:".cyan(),
        host.yellow(),
        ports.first().unwrap_or(&0),
        ports.last().unwrap_or(&0),
        timeout_ms
    );

    // Resolve host name to IP address(es)
    let target = format!("{}:80", host); // Add dummy port for resolution
    let addrs: Vec<SocketAddr> = match target.to_socket_addrs() {
        Ok(iter) => iter.collect(),
        Err(e) => return Err(format!("Failed to resolve host '{}': {}", host, e).into()),
    };

    if addrs.is_empty() {
        return Err(format!("Could not resolve '{}' to any IP address", host).into());
    }

    // Use the first resolved IP address
    let ip_addr = addrs[0].ip();
    println!("Resolved '{}' to IP: {}", host.yellow(), ip_addr.to_string().cyan());
    println!("{}", "Scanning...".dimmed());

    let timeout = Duration::from_millis(timeout_ms);
    let mut open_ports = Vec::new();

    for &port in ports {
        let socket_addr = SocketAddr::new(ip_addr, port);
        match TcpStream::connect_timeout(&socket_addr, timeout) {
            Ok(_) => {
                println!("  Port {}: {}", port.to_string().yellow(), "Open".green());
                open_ports.push(port);
                // Stream is dropped here, closing the connection
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::ConnectionRefused => {
                // Expected errors for closed ports - do nothing
            }
            Err(e) => {
                 // Unexpected error
                eprintln!("  Error scanning port {}: {}", port, e.to_string().red());
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if open_ports.is_empty() {
        println!("{}", "Scan complete. No open ports found in the specified range.".dimmed());
    } else {
        println!("Scan complete. Found {} open port(s): {}",
                 open_ports.len().to_string().green(),
                 open_ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ").yellow());
    }

    Ok(())
} 