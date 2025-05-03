//! src/net_tools.rs
use colored::*;
use dns_lookup::lookup_addr;
use futures::{stream::FuturesUnordered, StreamExt};
use get_if_addrs::{get_if_addrs, IfAddr};
use ipnetwork::Ipv4Network;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpStream, time};

// Device information structure
#[derive(Clone, Debug, Default)]
struct DeviceInfo {
    hostname: String,
    mac_address: Option<String>,
    open_ports: Vec<u16>,
    manufacturer: Option<String>,
    device_type: Option<String>,
}

/// ---------------------------------------------------------------------------
/// Helpers
/// ---------------------------------------------------------------------------

async fn port_is_open(addr: SocketAddr, timeout: Duration) -> bool {
    time::timeout(timeout, TcpStream::connect(addr)).await.is_ok()
}

/// ---------------------------------------------------------------------------
/// Bandwidth monitoring
/// ---------------------------------------------------------------------------

/// Provides information about the current network bandwidth.
/// 
/// This is a placeholder implementation since proper bandwidth monitoring
/// requires platform-specific implementations.
pub async fn get_bandwidth_snapshot() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("{}", "Network bandwidth monitoring is temporarily unavailable.".yellow());
    println!("{}", "This feature requires additional system access that isn't currently enabled.".dimmed());
    println!("{}", "Use the port scanning option instead for network operations.".dimmed());
    Ok(())
}

/// ---------------------------------------------------------------------------
/// Device discovery
/// ---------------------------------------------------------------------------

/// Scan every directly-connected IPv4 network for live hosts.
///
/// A "live" host is any address that responds on common ports (22, 80, 443, 3389, etc.).
/// Enhanced to display detailed device information including MAC addresses,
/// device types, and manufacturers when possible.
pub async fn discover_network_devices(timeout_ms: u64) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("{}", "🔍  Discovering network devices...".cyan().bold());
    println!("{}", "This will scan your local networks for connected devices".dimmed());

    // 1. Build the set of IPv4 networks we should test.
    let mut nets: BTreeSet<Ipv4Network> = BTreeSet::new();
    let mut local_ips = Vec::new();
    
    println!("{}", "Detecting network interfaces...".cyan());
    for iface in get_if_addrs().map_err(|e| -> Box<dyn Error + Send + Sync> { Box::new(e) })? {
        if iface.is_loopback() {
            continue;
        }
        if let IfAddr::V4(v4) = iface.addr {
            // Create network using CIDR prefix instead of netmask
            let prefix_len = netmask_to_prefix(v4.netmask);
            let net = Ipv4Network::new(v4.ip, prefix_len)
                .unwrap_or_else(|_| Ipv4Network::new(v4.ip, 24).unwrap());
            
            println!("  {} Interface: {} - IP: {} - Network: {}/{}", 
                "✓".green(),
                iface.name.cyan(), 
                v4.ip.to_string().yellow(),
                net.ip().to_string(),
                net.prefix()
            );
            
            local_ips.push(v4.ip);
            nets.insert(net);
        }
    }
    if nets.is_empty() {
        return Err("No routable IPv4 interface found".into());
    }

    // Print a separator
    println!("{}", "─────────────────────────────────────────────────────────────".dimmed());
    
    for net in nets {
        // Skip small networks like /31 and /32
        if net.prefix() >= 31 {
            continue;
        }
        
        println!(
            "{} {}  ({} potential hosts)",
            "📡  Scanning Network:".cyan().bold(),
            net.to_string().yellow().bold(),
            (net.size() - 2).to_string().green()
        );
        scan_subnet(net, timeout_ms).await?;
    }
    Ok(())
}

// Helper function to convert an IPv4 netmask to a prefix length (e.g., 255.255.255.0 -> 24)
fn netmask_to_prefix(netmask: Ipv4Addr) -> u8 {
    let octets = netmask.octets();
    let mut count = 0;
    for octet in octets.iter() {
        count += octet.count_ones();
    }
    count as u8
}

async fn scan_subnet(net: Ipv4Network, timeout_ms: u64) -> Result<(), Box<dyn Error + Send + Sync>> {
    let timeout = Duration::from_millis(timeout_ms);
    let ports = [22, 80, 443, 3389, 8080, 8443];
    let live = Arc::new(Mutex::new(BTreeMap::<Ipv4Addr, DeviceInfo>::new()));
    let mut tasks = FuturesUnordered::new();

    // Get MAC address cache from arp table (for faster device identification)
    let mac_cache = get_arp_cache();
    
    // Manually iterate through IP addresses in the network
    let start_ip = u32::from(net.network());
    let end_ip = start_ip + net.size() - 2; // Skip network and broadcast addresses
    
    println!("{}", "Scanning network, please wait...".dimmed());
    
    for ip_int in start_ip+1..=end_ip {
        let host = Ipv4Addr::from(ip_int);
        let live = live.clone();
        let mac_cache = mac_cache.clone();
        
        tasks.push(tokio::spawn(async move {
            let mut detected_ports = Vec::new();
            
            for &p in &ports {
                if port_is_open(SocketAddr::new(IpAddr::V4(host), p), timeout).await {
                    detected_ports.push(p);
                }
            }
            
            if !detected_ports.is_empty() {
                let name = lookup_addr(&IpAddr::V4(host)).unwrap_or_else(|_| "Unknown".into());
                
                // Get MAC address from cache if available
                let mac_address = mac_cache.get(&host).cloned();
                
                // Try to guess device type based on open ports and hostname
                let device_type = guess_device_type(&name, &detected_ports);
                
                // Guess manufacturer from MAC address if available
                let manufacturer = match &mac_address {
                    Some(mac) => guess_manufacturer(mac),
                    None => None,
                };
                
                let device_info = DeviceInfo {
                    hostname: name.clone(),
                    mac_address,
                    open_ports: detected_ports.clone(),
                    manufacturer,
                    device_type,
                };
                
                let mut map = live.lock().unwrap();
                if map.insert(host, device_info.clone()).is_none() {
                    println!("  {} {} - {}",
                        "✓".green(),
                        host.to_string().cyan(),
                        name.yellow()
                    );
                }
            }
        }));
    }
    
    while tasks.next().await.is_some() {}

    // --- summary ------------------------------------------------------------
    let map = live.lock().unwrap();
    if map.is_empty() {
        println!("{}", "No live devices found.\n".yellow());
    } else {
        println!(
            "\n{} {} device(s) discovered on network:",
            "✔  Complete –".green(),
            map.len().to_string().bold()
        );
        
        println!("{}", "╭───────────────────────────────────────────────────────────────────────╮".cyan());
        println!("{:<4} {:<15} {:<20} {:<15} {:<18} {}", 
            "│ #".cyan(),
            "│ IP Address".cyan(), 
            "│ Hostname".cyan(), 
            "│ Device Type".cyan(),
            "│ Manufacturer".cyan(),
            "│ Ports".cyan()
        );
        println!("{}", "├───────────────────────────────────────────────────────────────────────┤".cyan());
        
        for (i, (ip, device)) in map.iter().enumerate() {
            let ports_str = if device.open_ports.is_empty() { 
                "N/A".to_string() 
            } else { 
                device.open_ports.iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ") 
            };
            
            // Format the MAC address for display, if available
            let mac_display = match &device.mac_address {
                Some(mac) => truncate(mac, 18),
                None => "Unknown".to_string()
            };
            
            println!("{:<4} {:<15} {:<20} {:<15} {:<18} {} {}",
                format!("│ {}", i+1).cyan(),
                format!("│ {}", ip).cyan(), 
                format!("│ {}", truncate(&device.hostname, 18)),
                format!("│ {}", device.device_type.as_deref().unwrap_or("Unknown")),
                format!("│ {}", device.manufacturer.as_deref().unwrap_or("Unknown")),
                format!("│ {}", ports_str),
                format!("│ MAC: {}", mac_display).dimmed()
            );
        }
        println!("{}", "╰───────────────────────────────────────────────────────────────────────╯".cyan());
        println!();
    }
    Ok(())
}

// Helper functions for device identification

// Get MAC addresses from ARP cache
fn get_arp_cache() -> HashMap<Ipv4Addr, String> {
    let mut result = HashMap::new();
    
    // Try to run arp command to get MAC addresses
    let output = match Command::new("arp").arg("-a").output() {
        Ok(output) => output,
        Err(_) => return result,
    };
    
    if !output.status.success() {
        return result;
    }
    
    let arp_output = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => return result,
    };
    
    // Parse the ARP table output (format differs by OS)
    for line in arp_output.lines() {
        // Skip header lines
        if line.contains("Address") || line.trim().is_empty() {
            continue;
        }
        
        // Extract IP and MAC address using common patterns
        if let Some(ip_str) = line.split_whitespace().find(|s| s.contains('.')) {
            if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                // Extract MAC address (format may vary by OS)
                if let Some(mac) = line.split_whitespace()
                    .find(|s| s.contains(':') || s.contains('-')) {
                    result.insert(ip, mac.to_string());
                }
            }
        }
    }
    
    result
}

// Guess device type based on hostname and open ports
fn guess_device_type(hostname: &str, open_ports: &[u16]) -> Option<String> {
    let hostname_lower = hostname.to_lowercase();
    
    // Check hostname for device type indicators
    if hostname_lower.contains("iphone") || hostname_lower.contains("ipad") {
        return Some("Apple Mobile".to_string());
    } else if hostname_lower.contains("android") {
        return Some("Android Device".to_string());
    } else if hostname_lower.contains("macbook") || hostname_lower.contains("mac") {
        return Some("Mac Computer".to_string());
    } else if hostname_lower.contains("windows") || hostname_lower.contains("pc") {
        return Some("Windows PC".to_string());
    } else if hostname_lower.contains("linux") || hostname_lower.contains("ubuntu") {
        return Some("Linux Host".to_string());
    } else if hostname_lower.contains("router") || hostname_lower.contains("gateway") {
        return Some("Router".to_string());
    } else if hostname_lower.contains("printer") {
        return Some("Printer".to_string());
    } else if hostname_lower.contains("camera") || hostname_lower.contains("cam") {
        return Some("Camera".to_string());
    }
    
    // Check open ports for device type indicators
    if open_ports.contains(&22) {
        return Some("SSH Server".to_string());
    } else if open_ports.contains(&80) || open_ports.contains(&443) || open_ports.contains(&8080) || open_ports.contains(&8443) {
        return Some("Web Server".to_string());
    } else if open_ports.contains(&3389) {
        return Some("RDP Server".to_string());
    }
    
    None
}

// Guess manufacturer from MAC address
fn guess_manufacturer(mac: &str) -> Option<String> {
    // Extract OUI (first 6 characters of MAC address without separators)
    let clean_mac = mac.replace(':', "").replace('-', "");
    if clean_mac.len() < 6 {
        return None;
    }
    
    let oui = clean_mac[0..6].to_uppercase();
    
    // Very simple OUI to manufacturer mapping for common vendors
    match oui.as_str() {
        "001122" | "003342" | "0050B6" => Some("Apple".to_string()),
        "FCFBFB" | "8C8ABE" => Some("Google".to_string()),
        "000DE8" | "00127F" => Some("Cisco".to_string()),
        "00E04C" | "002255" => Some("Dell".to_string()),
        "000C29" | "005056" | "001C14" => Some("VMware".to_string()),
        "00D0B7" | "001150" => Some("Intel".to_string()),
        "C8D719" | "00E068" => Some("Samsung".to_string()),
        "001320" | "74D435" => Some("HP".to_string()),
        "001E10" | "002618" => Some("D-Link".to_string()),
        "0026F2" | "7C2664" => Some("NETGEAR".to_string()),
        "D0608C" | "747548" => Some("TP-Link".to_string()),
        _ => None,
    }
}

// Truncate string to specific length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}..", &s[0..max_len-2])
    }
}

/// ---------------------------------------------------------------------------
/// Ping tool
/// ---------------------------------------------------------------------------

/// Pings a host to check if it's online and measures response time
pub async fn ping_host(target: &str, count: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("{}", format!("🔔 Pinging {} {} times...", target, count).cyan().bold());
    
    // Clone target string for use in the async block
    let target_owned = target.to_string();
    
    // Check if host is valid by attempting to resolve it
    let ip_addr = match target.parse::<Ipv4Addr>() {
        Ok(ip) => ip.to_string(),
        Err(_) => {
            // Try to resolve hostname to IP
            let target_clone = target_owned.clone();
            match tokio::task::spawn_blocking(move || -> Result<String, Box<dyn Error + Send + Sync>> {
                let ips = dns_lookup::lookup_host(&target_clone)?
                    .into_iter()
                    .filter(|ip| ip.is_ipv4())
                    .collect::<Vec<_>>();
                
                if ips.is_empty() {
                    return Err(format!("Could not resolve hostname: {}", target_clone).into());
                }
                Ok(ips[0].to_string())
            }).await?? {
                ip => ip,
            }
        }
    };
    
    println!("{}", format!("Resolved to IP: {}", ip_addr).dimmed());
    
    let mut success_count = 0;
    let mut _failure_count = 0; // Using underscore prefix to indicate intentionally unused
    let mut total_ms: f64 = 0.0;
    let mut min_ms: f64 = f64::MAX;
    let mut max_ms: f64 = 0.0;
    
    // Convert values to strings before using in vectors
    let count_str = count.to_string();
    
    // Platform-specific ping command
    let (command, args) = if cfg!(target_os = "windows") {
        ("ping", vec!["-n", &count_str, &ip_addr])
    } else {
        // macOS and Linux use -c flag
        ("ping", vec!["-c", &count_str, &ip_addr])
    };
    
    // Execute ping command
    let output = match Command::new(command)
        .args(&args)
        .output() {
        Ok(output) => output,
        Err(e) => return Err(format!("Failed to execute ping command: {}", e).into()),
    };
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    // Print raw output first
    println!("{}", "-".repeat(50).dimmed());
    println!("{}", output_str);
    println!("{}", "-".repeat(50).dimmed());
    
    // Parse output to extract times (varies by OS, this is a simplified version)
    for line in output_str.lines() {
        // Check for successful ping response
        if (line.contains("bytes from") || line.contains("Reply from")) && 
           (line.contains("time=") || line.contains("time<") || line.contains("time:")) {
            success_count += 1;
            
            // Extract time in ms - this parsing is simplified and may need adjustment
            if let Some(time_pos) = line.find("time=") {
                if let Some(end_pos) = line[time_pos+5..].find(" ") {
                    if let Ok(time_ms) = line[time_pos+5..time_pos+5+end_pos].trim_end_matches("ms").parse::<f64>() {
                        total_ms += time_ms;
                        min_ms = min_ms.min(time_ms);
                        max_ms = max_ms.max(time_ms);
                    }
                }
            } else if let Some(time_pos) = line.find("time:") {
                if let Some(end_pos) = line[time_pos+5..].find(" ") {
                    if let Ok(time_ms) = line[time_pos+5..time_pos+5+end_pos].trim_end_matches("ms").parse::<f64>() {
                        total_ms += time_ms;
                        min_ms = min_ms.min(time_ms);
                        max_ms = max_ms.max(time_ms);
                    }
                }
            }
        } else if line.contains("Request timed out") || line.contains("Destination Host Unreachable") {
            _failure_count += 1;
        }
    }
    
    // Calculate statistics
    println!("{}", "Ping Statistics Summary:".blue().bold());
    println!("Target: {}", target.yellow());
    println!("Packets: Sent = {}, Received = {}, Lost = {} ({}% loss)",
        count,
        success_count,
        count - success_count,
        ((count - success_count) as f64 / count as f64 * 100.0).round()
    );
    
    if success_count > 0 {
        let avg_ms = total_ms / success_count as f64;
        println!("Round-trip times: Min = {:.2}ms, Max = {:.2}ms, Average = {:.2}ms", 
            min_ms, 
            max_ms,
            avg_ms
        );
    }
    
    Ok(())
}

/// ---------------------------------------------------------------------------
/// Port scanner
/// ---------------------------------------------------------------------------

/// Scan `ports` on `target` (hostname or IPv4) within `timeout_ms` per port.
pub async fn scan_ports(target: &str, ports: &[u16], timeout_ms: u64) -> Result<(), Box<dyn Error + Send + Sync>> {
    let timeout = Duration::from_millis(timeout_ms);

    // 1. Resolve once
    let ip = format!("{}:0", target)
        .to_socket_addrs()
        .map_err(|e| -> Box<dyn Error + Send + Sync> { Box::new(e) })?
        .find(|a| a.is_ipv4())
        .map(|a| a.ip())
        .ok_or_else(|| -> Box<dyn Error + Send + Sync> { "Failed to resolve host".into() })?;

    println!(
        "{} {} ({}) – timeout {} ms",
        "🚀  Port scan on".cyan(),
        target.yellow(),
        ip.to_string().cyan(),
        timeout_ms
    );

    // 2. Concurrent scan
    let open = Arc::new(Mutex::new(Vec::<u16>::new()));
    let mut tasks = FuturesUnordered::new();

    for &port in ports {
        let open = open.clone();
        tasks.push(tokio::spawn(async move {
            if port_is_open(SocketAddr::new(ip, port), timeout).await {
                open.lock().unwrap().push(port);
            }
        }));
    }
    while tasks.next().await.is_some() {}

    // 3. Report
    let mut open = open.lock().unwrap();
    open.sort_unstable();

    if open.is_empty() {
        println!("{}", "No open ports detected.".yellow());
    } else {
        println!(
            "{} {}",
            "✓  Open port(s):".green(),
            open.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ").yellow()
        );
    }
    Ok(())
}
