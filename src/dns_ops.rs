use crate::cli::DnsAction;
use colored::*;
use std::error::Error;
use std::process::{Command, Output};

pub async fn manage_dns(action: DnsAction) -> Result<(), Box<dyn Error + Send + Sync>> {
    match action {
        DnsAction::Flush => flush_dns_cache(),
        // DnsAction::View => view_dns_cache(), // Placeholder if view is added later
    }
}

fn flush_dns_cache() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("{}", "Attempting to flush DNS cache...".cyan());
    println!("{}", "Note: This operation usually requires administrator/sudo privileges.".yellow());

    let output_result: Result<Output, std::io::Error>;

    #[cfg(target_os = "windows")]
    {
        println!("Running: {}", "ipconfig /flushdns".dimmed());
        output_result = Command::new("ipconfig").arg("/flushdns").output();
    }

    #[cfg(target_os = "macos")]
    {
        println!("Running: {}", "sudo dscacheutil -flushcache".dimmed());
        let out1 = Command::new("sudo").arg("dscacheutil").arg("-flushcache").output();
        if out1.is_err() || !out1.as_ref().unwrap().status.success() {
             println!("{}", "dscacheutil failed. Trying killall mDNSResponder...".yellow());
             println!("Running: {}", "sudo killall -HUP mDNSResponder".dimmed());
             output_result = Command::new("sudo").arg("killall").arg("-HUP").arg("mDNSResponder").output();
        } else {
            // If dscacheutil worked, try killall as well for good measure (common practice)
            println!("Running: {}", "sudo killall -HUP mDNSResponder".dimmed());
            Command::new("sudo").arg("killall").arg("-HUP").arg("mDNSResponder").output()?;
            output_result = out1; // Report status of the primary command
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux DNS flushing is highly variable. Try systemd-resolved first.
        println!("Attempting flush with systemd-resolve...");
        println!("Running: {}", "sudo systemd-resolve --flush-caches".dimmed());
        let systemd_output = Command::new("sudo").arg("systemd-resolve").arg("--flush-caches").output();

        if systemd_output.is_ok() && systemd_output.as_ref().unwrap().status.success() {
            output_result = systemd_output;
        } else {
            println!("{}", "systemd-resolve failed or not found. Trying nscd...".yellow());
            println!("Running: {}", "sudo /etc/init.d/nscd restart".dimmed());
            let nscd_output = Command::new("sudo").arg("/etc/init.d/nscd").arg("restart").output();
            if nscd_output.is_ok() && nscd_output.as_ref().unwrap().status.success() {
                output_result = nscd_output;
            } else {
                 println!("{}", "nscd failed or not found. Trying dnsmasq...".yellow());
                 println!("Running: {}", "sudo service dnsmasq restart".dimmed());
                 // Fallback to dnsmasq as the final attempt
                 output_result = Command::new("sudo").arg("service").arg("dnsmasq").arg("restart").output();
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        println!("{}", "DNS flush command not known for this OS.".red());
        return Err("Unsupported OS for DNS flush".into());
    }

    match output_result {
        Ok(output) => {
            if output.status.success() {
                println!("{}", "DNS cache flush command executed successfully.".green());
                // Print stdout/stderr only if there's potentially useful info
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.trim().is_empty() {
                    println!("Output:\n{}", stdout.dimmed());
                }
                Ok(())
            } else {
                eprintln!("{}", "DNS cache flush command failed.".red());
                eprintln!("Exit Code: {}", output.status);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    eprintln!("Error Output:\n{}", stderr.red());
                }
                 // Try to give a hint about sudo
                 if stderr.to_lowercase().contains("permission denied") || stderr.to_lowercase().contains("not permitted") || output.status.code() == Some(1) {
                     eprintln!("{}", "Hint: Did you run the application with administrator/sudo privileges?".yellow());
                 }
                Err(format!("Flush command failed with status: {}", output.status).into())
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Failed to execute DNS flush command".red(), e);
            // Check if the error is because the command wasn't found (e.g., sudo missing)
            if e.kind() == std::io::ErrorKind::NotFound {
                 eprintln!("{}", "Hint: Ensure required commands (like sudo, ipconfig, etc.) are installed and in your PATH.".yellow());
            }
            Err(e.into())
        }
    }
}

// Placeholder for viewing cache - complex and OS-specific
// fn view_dns_cache() -> Result<(), Box<dyn Error + Send + Sync>> {
//     println!("{}", "Viewing DNS cache is not implemented yet.".yellow());
//     Ok(())
// } 