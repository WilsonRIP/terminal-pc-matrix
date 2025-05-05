use anyhow::Result;
use colored::*;
use sysinfo::{System, Disks, Networks};
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, UNIX_EPOCH};
use std::fmt;

/// Structure to hold the full system information
#[derive(Debug)]
pub struct SystemInfo {
    hostname: String,
    os_name: String,
    os_version: String,
    kernel_version: String,
    total_memory: u64,
    used_memory: u64,
    total_swap: u64,
    used_swap: u64,
    uptime: Duration,
    boot_time: Duration,
    processors: Vec<ProcessorInfo>,
    disks: Vec<DiskInfo>,
    networks: Vec<NetworkInfo>,
}

#[derive(Debug)]
struct ProcessorInfo {
    name: String,
    brand: String, 
    frequency: u64,
    vendor_id: String,
    cores: usize,
}

#[derive(Debug)]
struct DiskInfo {
    name: String,
    mount_point: String,
    file_system: String,
    total_space: u64,
    available_space: u64,
    is_removable: bool,
}

#[derive(Debug)]
struct NetworkInfo {
    name: String,
    sent_bytes: u64,
    received_bytes: u64,
    packets_sent: u64,
    packets_received: u64,
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "=== SYSTEM INFORMATION ===".cyan().bold())?;
        writeln!(f, "{}: {}", "Hostname".green(), self.hostname)?;
        writeln!(f, "{}: {} {}", "OS".green(), self.os_name, self.os_version)?;
        writeln!(f, "{}: {}", "Kernel".green(), self.kernel_version)?;
        writeln!(f, "{}: {} days, {} hours, {} minutes", 
            "Uptime".green(),
            self.uptime.as_secs() / 86400,
            (self.uptime.as_secs() % 86400) / 3600,
            (self.uptime.as_secs() % 3600) / 60)?;
        
        writeln!(f, "\n{}", "=== MEMORY ===".cyan().bold())?;
        writeln!(f, "{}: {:.2} GB / {:.2} GB ({:.1}%)", 
            "Memory Usage".green(),
            self.used_memory as f64 / 1_073_741_824.0,
            self.total_memory as f64 / 1_073_741_824.0,
            (self.used_memory as f64 / self.total_memory as f64) * 100.0)?;
        writeln!(f, "{}: {:.2} GB / {:.2} GB ({:.1}%)", 
            "Swap Usage".green(),
            self.used_swap as f64 / 1_073_741_824.0,
            self.total_swap as f64 / 1_073_741_824.0,
            if self.total_swap > 0 { (self.used_swap as f64 / self.total_swap as f64) * 100.0 } else { 0.0 })?;
        
        writeln!(f, "\n{}", "=== PROCESSORS ===".cyan().bold())?;
        for (i, proc) in self.processors.iter().enumerate() {
            writeln!(f, "{} {}: {}", "CPU".green(), i + 1, proc.name)?;
            writeln!(f, "  {}: {}", "Brand".yellow(), proc.brand)?;
            writeln!(f, "  {}: {} MHz", "Frequency".yellow(), proc.frequency)?;
            writeln!(f, "  {}: {}", "Vendor ID".yellow(), proc.vendor_id)?;
            writeln!(f, "  {}: {}", "Cores".yellow(), proc.cores)?;
        }
        
        writeln!(f, "\n{}", "=== DISKS ===".cyan().bold())?;
        for disk in &self.disks {
            writeln!(f, "{}: {} ({})", "Disk".green(), disk.name, 
                if disk.is_removable { "Removable".italic() } else { "Fixed".italic() })?;
            writeln!(f, "  {}: {}", "Mount Point".yellow(), disk.mount_point)?;
            writeln!(f, "  {}: {}", "File System".yellow(), disk.file_system)?;
            writeln!(f, "  {}: {:.2} GB / {:.2} GB ({:.1}%)", 
                "Space".yellow(),
                (disk.total_space - disk.available_space) as f64 / 1_073_741_824.0,
                disk.total_space as f64 / 1_073_741_824.0,
                if disk.total_space > 0 { 
                    ((disk.total_space - disk.available_space) as f64 / disk.total_space as f64) * 100.0 
                } else { 
                    0.0 
                })?;
        }
        
        writeln!(f, "\n{}", "=== NETWORK INTERFACES ===".cyan().bold())?;
        for net in &self.networks {
            writeln!(f, "{}: {}", "Interface".green(), net.name)?;
            writeln!(f, "  {}: {} MB", "Data Sent".yellow(), net.sent_bytes / 1_048_576)?;
            writeln!(f, "  {}: {} MB", "Data Received".yellow(), net.received_bytes / 1_048_576)?;
            writeln!(f, "  {}: {}", "Packets Sent".yellow(), net.packets_sent)?;
            writeln!(f, "  {}: {}", "Packets Received".yellow(), net.packets_received)?;
        }
        
        Ok(())
    }
}

/// Gather all system information
pub fn get_system_info() -> Result<SystemInfo> {
    // Create a new System instance
    let mut system = System::new_all();
    
    // Refresh all information
    system.refresh_all();
    
    // Basic system info
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".into());
    let os_name = System::name().unwrap_or_else(|| "Unknown".into());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".into());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".into());
    
    // Memory info
    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    let total_swap = system.total_swap();
    let used_swap = system.used_swap();
    
    // System uptime and boot time
    let uptime = Duration::from_secs(System::uptime());
    
    // Calculate boot time by subtracting uptime from current time
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let boot_time = if now > uptime {
        now - uptime
    } else {
        Duration::from_secs(0)
    };
    
    // Processor info
    let processors = system.cpus().iter().map(|p| {
        ProcessorInfo {
            name: p.name().to_string(),
            brand: p.brand().to_string(),
            frequency: p.frequency() as u64,
            vendor_id: p.vendor_id().to_string(),
            cores: system.physical_core_count().unwrap_or(0),
        }
    }).collect();
    
    // Get disk information
    let disks_info = Disks::new_with_refreshed_list();
    let disks = disks_info.iter().map(|d| {
        DiskInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_string_lossy().to_string(),
            file_system: d.file_system().to_string_lossy().to_string(),
            total_space: d.total_space(),
            available_space: d.available_space(),
            is_removable: d.is_removable(),
        }
    }).collect();
    
    // Get network information
    let networks_info = Networks::new_with_refreshed_list();
    let networks = networks_info.iter().map(|(name, data)| {
        NetworkInfo {
            name: name.clone(),
            sent_bytes: data.total_transmitted(),
            received_bytes: data.total_received(),
            packets_sent: data.total_packets_transmitted(),
            packets_received: data.total_packets_received(),
        }
    }).collect();
    
    Ok(SystemInfo {
        hostname,
        os_name,
        os_version,
        kernel_version,
        total_memory,
        used_memory,
        total_swap,
        used_swap,
        uptime,
        boot_time,
        processors,
        disks,
        networks,
    })
}

/// Display all system information on the console
pub fn display_system_info() -> Result<()> {
    let system_info = get_system_info()?;
    println!("{}", system_info);
    Ok(())
}

/// Save system information to a file
pub fn save_system_info_to_file(path: &Path) -> Result<()> {
    let system_info = get_system_info()?;
    
    // Create or truncate the file
    let mut file = File::create(path)?;
    
    // Write system info as formatted text
    write!(file, "{}", system_info)?;
    
    println!("{} {}", "System information saved to:".green(), path.display());
    Ok(())
}

/// Format size in bytes to human-readable format
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.2} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} bytes", size)
    }
}

/// Get system information as a formatted String
/// This version is suitable for GUI display where color codes are not needed.
pub fn get_system_info_string() -> Result<String> {
    let system_info = get_system_info()?;
    // Use the Display implementation to format the string
    Ok(format!("{}", system_info))
}

pub fn handle_pc_specs_command(args: crate::cli::PCSpecsArgs) -> anyhow::Result<()> {
    if let Some(output_path) = args.output {
        // Save to file
        save_system_info_to_file(&output_path)
    } else {
        // Display to console
        display_system_info()
    }
} 