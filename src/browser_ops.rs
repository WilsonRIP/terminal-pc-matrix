use colored::*;
use std::process;

// Function to close all major web browsers
#[cfg(target_os = "macos")]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Attempting to close browsers on macOS...".cyan());
    let browsers = [
        "Safari",
        "Google Chrome",
        "Firefox",
        "Microsoft Edge",
        "Brave Browser",
        "Opera",
        "Opera GX",
        "Zen",
        "Google Chrome Beta",
        "Microsoft Edge Beta",
    ];
    let mut errors = Vec::new();

    for browser in browsers {
        let command = format!("osascript -e 'quit app \"{}\"'", browser);
        println!("Running: {}", command.dimmed());
        match process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .status()
        {
            Ok(status) => {
                if status.success() {
                    println!("  Closed {}", browser.green());
                } else {
                    println!("  'quit {}' finished (app likely not running)", browser.dimmed());
                }
            }
            Err(e) => {
                let err_msg = format!("Failed to execute command for {}: {}", browser, e);
                eprintln!("{}", err_msg.red());
                errors.push(err_msg);
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if errors.is_empty() {
        println!("{}", "Finished attempting to close browsers.".green());
        Ok(())
    } else {
        println!("{}", format!("Finished with {} error(s).", errors.len()).yellow());
        Err(errors.join("\n").into())
    }
}

#[cfg(target_os = "windows")]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Attempting to close browsers on Windows...".cyan());
    let browsers = [
        "chrome.exe",
        "firefox.exe",
        "msedge.exe",
        "ieplore.exe", // Assuming this was a typo for iexplore.exe, check if correct
        "iexplore.exe",
        "brave.exe",
        "opera.exe",
        "launcher.exe",
        "zen.exe",
    ];
    let mut errors = Vec::new();

    for browser in browsers {
        let command = format!("taskkill /F /IM {}", browser);
        println!("Running: {}", command.dimmed());
        match process::Command::new("cmd")
            .args(&["/C", &command])
            .status()
        {
             Ok(status) => {
                if status.code() == Some(0) {
                    println!("  Closed {}", browser.green());
                } else if status.code() == Some(128) {
                     println!("  '{}' was not running.", browser.dimmed());
                } else {
                     println!("  'taskkill' for {} finished with code: {:?} (may indicate error or already closed)", browser.yellow(), status.code());
                 }
            }
            Err(e) => {
                let err_msg = format!("Failed to execute taskkill for {}: {}", browser, e);
                eprintln!("{}", err_msg.red());
                errors.push(err_msg);
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if errors.is_empty() {
        println!("{}", "Finished attempting to close browsers.".green());
        Ok(())
    } else {
        println!("{}", format!("Finished with {} error(s).", errors.len()).yellow());
        Err(errors.join("\n").into())
    }
}

#[cfg(target_os = "linux")]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Attempting to close browsers on Linux...".cyan());
    let browsers = [
        "chrome",
        "google-chrome",
        "firefox",
        "msedge",
        "brave",
        "opera",
        "opera-gx",
        "zen",
        "google-chrome-beta",
        "microsoft-edge-beta",
    ];
    let mut errors = Vec::new();

    for browser in browsers {
        let command = format!("killall {}", browser);
        println!("Running: {}", command.dimmed());
        match process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .status()
         {
            Ok(status) => {
                if status.success() {
                    println!("  Killed process(es) named {}", browser.green());
                } else {
                    println!("  'killall {}' finished (process likely not running)", browser.dimmed());
                }
            }
            Err(e) => {
                 let err_msg = format!("Failed to execute killall for {}: {}", browser, e);
                 eprintln!("{}", err_msg.red());
                 errors.push(err_msg);
            }
        }
    }
    println!("{}", "-".repeat(40).dimmed());
     if errors.is_empty() {
        println!("{}", "Finished attempting to close browsers.".green());
        Ok(())
    } else {
        println!("{}", format!("Finished with {} error(s).", errors.len()).yellow());
        Err(errors.join("\n").into())
    }
}

// Fallback for unsupported OS
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error>> {
    let os_name = std::env::consts::OS;
    eprintln!(
        "{}",
        format!("Warning: Closing browsers is not supported on this OS: {}", os_name).yellow()
    );
    Err(format!("Unsupported OS: {}", os_name).into())
} 