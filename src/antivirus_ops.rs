use std::path::{Path, PathBuf};
use std::process::Command;
use colored::*;
use anyhow::Result;
use std::time::Duration;
use std::fs;
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;
use lazy_static::lazy_static;

/// Represents a virus scan result
#[derive(Debug)]
pub struct ScanResult {
    pub path: PathBuf,
    pub status: ScanStatus,
    pub threat_name: Option<String>,
}

/// Status of a virus scan
#[derive(Debug, PartialEq)]
pub enum ScanStatus {
    Clean,
    Infected,
    Error,
    Skipped,
}

/// Check if ClamAV is installed on the system
pub fn check_clamav_installed() -> bool {
    match Command::new("clamscan")
        .arg("--version")
        .output() {
            Ok(_) => true,
            Err(_) => false,
    }
}

/// Get ClamAV version information
pub fn get_clamav_info() -> Result<String> {
    let output = Command::new("clamscan")
        .arg("--version")
        .output()?;
    
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    } else {
        Err(anyhow::anyhow!("Failed to get ClamAV version"))
    }
}

/// Update ClamAV virus definitions
pub fn update_virus_definitions() -> Result<String> {
    println!("{}", "Updating virus definitions...".cyan());
    
    // Use freshclam to update virus definitions
    let output = Command::new("freshclam")
        .output()?;
    
    if output.status.success() {
        let update_info = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(update_info)
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!("Failed to update virus definitions: {}", error))
    }
}

/// Scan a single file for viruses
pub fn scan_file(file_path: &Path) -> Result<ScanResult> {
    if !file_path.exists() {
        return Ok(ScanResult {
            path: file_path.to_path_buf(),
            status: ScanStatus::Error,
            threat_name: Some("File not found".to_string()),
        });
    }
    
    if !file_path.is_file() {
        return Ok(ScanResult {
            path: file_path.to_path_buf(),
            status: ScanStatus::Skipped,
            threat_name: Some("Not a file".to_string()),
        });
    }
    
    println!("{} {}", "Scanning file:".cyan(), file_path.display());
    
    let output = Command::new("clamscan")
        .arg("--no-summary")
        .arg(file_path)
        .output()?;
    
    // ClamAV returns exit code 1 when a virus is found
    if output.status.code() == Some(1) {
        // Parse output to extract virus name
        let stdout = String::from_utf8_lossy(&output.stdout);
        let threat_name = extract_threat_name(&stdout, file_path);
        
        Ok(ScanResult {
            path: file_path.to_path_buf(),
            status: ScanStatus::Infected,
            threat_name,
        })
    } else if output.status.success() {
        Ok(ScanResult {
            path: file_path.to_path_buf(), 
            status: ScanStatus::Clean,
            threat_name: None,
        })
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(ScanResult {
            path: file_path.to_path_buf(),
            status: ScanStatus::Error,
            threat_name: Some(format!("Scan error: {}", error)),
        })
    }
}

/// Scan a directory for viruses
pub fn scan_directory(dir_path: &Path, recursive: bool) -> Result<Vec<ScanResult>> {
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(anyhow::anyhow!("Invalid directory path"));
    }
    
    println!("{} {}", "Scanning directory:".cyan(), dir_path.display());
    
    let mut results = Vec::new();
    let mut file_count = 0;
    
    // Count files first for progress bar
    for entry in WalkDir::new(dir_path).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            file_count += 1;
        }
    }
    
    // Set up progress bar
    let pb = ProgressBar::new(file_count);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    // Build the command with the right arguments
    let mut cmd = Command::new("clamscan");
    cmd.arg("--no-summary");
    
    if recursive {
        cmd.arg("-r");
    }
    
    cmd.arg(dir_path);
    
    let output = cmd.output()?;
    
    // Process the output
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    // Parse the ClamAV output
    for line in stdout.lines() {
        if line.contains(": ") {
            let parts: Vec<&str> = line.splitn(2, ": ").collect();
            if parts.len() == 2 {
                let file_path_str = parts[0];
                let status_str = parts[1];
                
                let file_path = PathBuf::from(file_path_str);
                
                if status_str == "OK" {
                    results.push(ScanResult {
                        path: file_path,
                        status: ScanStatus::Clean,
                        threat_name: None,
                    });
                } else {
                    results.push(ScanResult {
                        path: file_path,
                        status: ScanStatus::Infected,
                        threat_name: Some(status_str.to_string()),
                    });
                }
                
                pb.inc(1);
            }
        }
    }
    
    pb.finish_with_message("Scan complete".green().to_string());
    
    Ok(results)
}

/// Extract threat name from ClamAV output
fn extract_threat_name(output: &str, file_path: &Path) -> Option<String> {
    for line in output.lines() {
        if line.contains(file_path.to_string_lossy().as_ref()) && line.contains("FOUND") {
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() >= 2 {
                return Some(parts[parts.len() - 2].to_string());
            }
        }
    }
    Some("Unknown threat".to_string())
}

/// Format scan results for display
pub fn format_scan_results(results: &[ScanResult]) -> String {
    let mut output = String::new();
    
    let clean_count = results.iter().filter(|r| r.status == ScanStatus::Clean).count();
    let infected_count = results.iter().filter(|r| r.status == ScanStatus::Infected).count();
    let error_count = results.iter().filter(|r| r.status == ScanStatus::Error).count();
    let skipped_count = results.iter().filter(|r| r.status == ScanStatus::Skipped).count();
    
    output.push_str(&format!("\n{} {} files scanned\n", "Summary:".green().bold(), results.len()));
    output.push_str(&format!("  {} {}\n", "Clean:".green(), clean_count));
    output.push_str(&format!("  {} {}\n", "Infected:".red(), infected_count));
    output.push_str(&format!("  {} {}\n", "Errors:".yellow(), error_count));
    output.push_str(&format!("  {} {}\n", "Skipped:".blue(), skipped_count));
    
    if infected_count > 0 {
        output.push_str(&format!("\n{}\n", "Infected Files:".red().bold()));
        for result in results.iter().filter(|r| r.status == ScanStatus::Infected) {
            output.push_str(&format!("  {} - {}\n", 
                result.path.display(), 
                result.threat_name.as_deref().unwrap_or("Unknown threat")));
        }
    }
    
    output
}

/// Quarantine an infected file
pub fn quarantine_file(file_path: &Path, quarantine_dir: &Path) -> Result<PathBuf> {
    // Create quarantine directory if it doesn't exist
    if !quarantine_dir.exists() {
        fs::create_dir_all(quarantine_dir)?;
    }
    
    // Generate quarantine file path
    let file_name = file_path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;
    
    let quarantine_file = quarantine_dir.join(file_name);
    
    // Move the file to quarantine
    fs::rename(file_path, &quarantine_file)?;
    
    Ok(quarantine_file)
} 