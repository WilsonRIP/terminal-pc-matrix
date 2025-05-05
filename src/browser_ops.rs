use colored::*;
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use glob::glob;
use dirs;

// Browser profile locations
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Brave,
    Opera,
    Vivaldi,
    Other(String),
}

// Browser data type
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserDataType {
    History,
    Cookies,
    Bookmarks,
    Passwords,
}

// Result of browser operation
#[derive(Debug)]
pub struct BrowserOpResult {
    pub success: bool,
    pub message: String,
    pub export_path: Option<PathBuf>,
}

/// Try to close (or kill) all major browsers on the host platform.
///
/// For browsers that are **not** running we just print a notice and continue;
/// we only return `Err` if the underlying shell / taskkill / killall command itself
/// cannot be executed.
///////////////////////////////////////////////////////////////////////////////
#[cfg(target_os = "macos")]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "Attempting to close browsers on macOS…".cyan());

    // macOS uses the Bundle name that appears in "Activity Monitor"
    let browsers = [
        "Safari",
        "Google Chrome",
        "Google Chrome Beta",
        "Google Chrome Canary",
        "Chromium",
        "Arc",
        "Brave Browser",
        "Vivaldi",
        "Firefox",
        "Firefox Developer Edition",
        "Firefox Nightly",
        "Microsoft Edge",
        "Microsoft Edge Beta",
        "Microsoft Edge Canary",
        "Opera",
        "Opera GX",
        "Tor Browser",
        "Orion",
        "Waterfox",
    ];

    let mut had_errors = false;

    for browser in browsers {
        let cmd = format!("osascript -e 'quit app \"{}\"'", browser);
        println!("Running: {}", cmd.dimmed());

        match Command::new("sh").arg("-c").arg(&cmd).status() {
            Ok(status) if status.success() => {
                println!("{} Closed {}", "✓".green(), browser.green());
            }
            Ok(_) => {
                // Non-zero exit code usually just means the app was not running.
                println!("  {} was not running.", browser.dimmed());
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    format!("Failed to execute osascript for {}: {}", browser, e).red()
                );
                had_errors = true;
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if had_errors {
        Err("One or more osascript calls failed".into())
    } else {
        println!("{}", "Finished attempting to close browsers.".green());
        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////////
#[cfg(target_os = "windows")]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "Attempting to close browsers on Windows…".cyan());

    // Executable names as they appear in Task Manager
    let browsers = [
        "chrome.exe",
        "chrome_beta.exe",
        "chrome_canary.exe",
        "msedge.exe",
        "msedgewebview2.exe",
        "firefox.exe",
        "vivaldi.exe",
        "brave.exe",
        "opera.exe",
        "opera_gx.exe",
        "arc.exe",
        "chromium.exe",
        "waterfox.exe",
        "tor.exe",
        "iexplore.exe",
    ];

    let mut had_errors = false;

    for browser in browsers {
        let cmd = format!("taskkill /F /IM {}", browser);
        println!("Running: {}", cmd.dimmed());

        match Command::new("cmd").args(&["/C", &cmd]).status() {
            Ok(status) if status.code() == Some(0) => {
                println!("{} Closed {}", "✓".green(), browser.green());
            }
            Ok(status) if status.code() == Some(128) || status.code() == Some(1) => {
                // 128 (or 1) → "process not found"
                println!("  {} was not running.", browser.dimmed());
            }
            Ok(status) => {
                println!(
                    "  taskkill for {} finished with exit code {:?}",
                    browser.yellow(),
                    status.code()
                );
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    format!("Failed to execute taskkill for {}: {}", browser, e).red()
                );
                had_errors = true;
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if had_errors {
        Err("One or more taskkill calls failed".into())
    } else {
        println!("{}", "Finished attempting to close browsers.".green());
        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////////
#[cfg(target_os = "linux")]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "Attempting to close browsers on Linux…".cyan());

    // Process names as they appear in `ps`
    let browsers = [
        "google-chrome",
        "google-chrome-beta",
        "google-chrome-unstable",
        "chromium",
        "chromium-browser",
        "brave",
        "brave-browser",
        "vivaldi",
        "vivaldi-snapshot",
        "firefox",
        "firefox-developer-edition",
        "librewolf",
        "waterfox",
        "tor-browser",
        "microsoft-edge",
        "microsoft-edge-beta",
        "opera",
        "opera-beta",
        "opera-developer",
        "arc", // (if/when Arc ships on Linux)
    ];

    let mut had_errors = false;

    for browser in browsers {
        let cmd = format!("killall {}", browser);
        println!("Running: {}", cmd.dimmed());

        match Command::new("sh").arg("-c").arg(&cmd).status() {
            Ok(status) if status.success() => {
                println!("{} Killed {}", "✓".green(), browser.green());
            }
            Ok(_) => {
                println!("  {} was not running.", browser.dimmed());
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    format!("Failed to execute killall for {}: {}", browser, e).red()
                );
                had_errors = true;
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if had_errors {
        Err("One or more killall calls failed".into())
    } else {
        println!("{}", "Finished attempting to close browsers.".green());
        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////////
// Fallback for everything else
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn close_browsers() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let os_name = std::env::consts::OS;
    eprintln!(
        "{}",
        format!("Warning: Closing browsers is not supported on this OS: {}", os_name).yellow()
    );
    Err(format!("Unsupported OS: {}", os_name).into())
}

// ----------------------------------- Browser Cleaner -----------------------------------

/// Returns the default profile directory for a given browser based on the OS.
/// Note: For Firefox, it attempts to find the *.default* or *.default-release* profile.
fn get_profile_dir(browser: BrowserType) -> Option<PathBuf> {
    let _home_dir = dirs::home_dir()?;
    // let home = PathBuf::from(home_dir); // Variable is unused

    #[cfg(target_os = "macos")]
    {
        let app_support = _home_dir.join("Library/Application Support");
        match browser {
            BrowserType::Chrome => Some(app_support.join("Google/Chrome/Default")),
            BrowserType::Firefox => {
                let profiles_path = app_support.join("Firefox/Profiles");
                find_firefox_profile_dir(&profiles_path)
            }
            BrowserType::Safari => Some(_home_dir.join("Library/Safari")),
            BrowserType::Edge => Some(app_support.join("Microsoft Edge/Default")),
            BrowserType::Brave => Some(app_support.join("BraveSoftware/Brave-Browser/Default")),
            BrowserType::Opera => Some(app_support.join("com.operasoftware.Opera")),
            BrowserType::Vivaldi => Some(app_support.join("Vivaldi/Default")),
            BrowserType::Other(_) => None,
        }
    }
    #[cfg(target_os = "linux")]
    {
        let config_dir = _home_dir.join(".config");
        match browser {
            BrowserType::Chrome => Some(config_dir.join("google-chrome/Default")),
            BrowserType::Firefox => {
                let profiles_path = _home_dir.join(".mozilla/firefox");
                find_firefox_profile_dir(&profiles_path)
            }
            BrowserType::Edge => Some(config_dir.join("microsoft-edge/Default")),
            BrowserType::Brave => Some(config_dir.join("BraveSoftware/Brave-Browser/Default")),
            BrowserType::Opera => Some(config_dir.join("opera")),
            BrowserType::Vivaldi => Some(config_dir.join("vivaldi/Default")),
            BrowserType::Safari => None, // Safari not on Linux
            BrowserType::Other(_) => None,
        }
    }
    #[cfg(target_os = "windows")]
    {
        let local_app_data = dirs::data_local_dir()?;
        let app_data = dirs::data_local_dir()?;
        match browser {
            BrowserType::Chrome => Some(local_app_data.join("Google/Chrome/User Data/Default")),
            BrowserType::Firefox => {
                let profiles_path = app_data.join("Mozilla/Firefox/Profiles");
                find_firefox_profile_dir(&profiles_path)
            }
            BrowserType::Edge => Some(local_app_data.join("Microsoft/Edge/User Data/Default")),
            BrowserType::Brave => Some(local_app_data.join("BraveSoftware/Brave-Browser/User Data/Default")),
            BrowserType::Opera => Some(app_data.join("Opera Software/Opera Stable")),
            BrowserType::Vivaldi => Some(local_app_data.join("Vivaldi/User Data/Default")),
            BrowserType::Safari => None, // Safari not really on Windows
            BrowserType::Other(_) => None,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None // Unsupported OS
    }
}

/// Helper to find the default Firefox profile directory.
fn find_firefox_profile_dir(profiles_path: &Path) -> Option<PathBuf> {
    if !profiles_path.exists() {
        return None;
    }
    // Look for directories ending with .default or .default-release
    let pattern = profiles_path.join("*.default*");
    glob(pattern.to_str()?).ok()?
        .filter_map(Result::ok)
        .find(|p| p.is_dir())
}

/// Gets the path to a specific data file within a browser's profile.
fn get_data_file_path(browser: &BrowserType, profile_dir: &Path, data_type: BrowserDataType) -> Option<PathBuf> {
    let filename = match data_type {
        BrowserDataType::History => match browser {
            BrowserType::Firefox => "places.sqlite",
            BrowserType::Safari => "History.db",
            _ => "History", // Chrome, Edge, Brave, Opera, Vivaldi
        },
        BrowserDataType::Cookies => match browser {
            BrowserType::Firefox => "cookies.sqlite",
            BrowserType::Safari => "Cookies.binarycookies",
            _ => "Cookies", // Chrome, Edge, Brave, Opera, Vivaldi
        },
        BrowserDataType::Bookmarks => match browser {
            BrowserType::Firefox => "places.sqlite", // History and Bookmarks are in the same file
            BrowserType::Safari => "Bookmarks.plist",
            _ => "Bookmarks", // Chrome, Edge, Brave, Opera, Vivaldi (JSON)
        },
        BrowserDataType::Passwords => match browser {
            BrowserType::Firefox => "logins.json", // Also needs key4.db potentially
            BrowserType::Safari => return None, // Uses Keychain
            _ => "Login Data", // Chrome, Edge, Brave, Opera, Vivaldi
        },
    };
    Some(profile_dir.join(filename))
}

/// Deletes browsing data for a specific browser.
pub fn delete_browser_data(browser: BrowserType, data_type: BrowserDataType) -> Result<BrowserOpResult, Box<dyn std::error::Error + Send + Sync>> {
    let profile_dir = get_profile_dir(browser.clone())
        .ok_or_else(|| format!("{:?} profile directory not found", browser))?;

    let data_file = get_data_file_path(&browser, &profile_dir, data_type.clone())
        .ok_or_else(|| format!("{:?} {:?} data file not supported or found", browser, data_type))?;

    if data_file.exists() {
        fs::remove_file(&data_file)?;
        let message = format!("Deleted {:?} {:?} at {}", browser, data_type, data_file.display());
        println!("{} {}", "✓".green(), message);
        Ok(BrowserOpResult { success: true, message, export_path: None })
    } else {
        let message = format!("{:?} {:?} file not found at {}", browser, data_type, data_file.display());
        Err(message.into())
    }
}

/// Exports browser data for a specific browser.
pub fn export_browser_data(browser: BrowserType, data_type: BrowserDataType) -> Result<BrowserOpResult, Box<dyn std::error::Error + Send + Sync>> {
    if matches!(data_type, BrowserDataType::History | BrowserDataType::Cookies) {
         return Err(format!("Export not supported for {:?}", data_type).into());
    }

    let profile_dir = get_profile_dir(browser.clone())
        .ok_or_else(|| format!("{:?} profile directory not found", browser))?;

    let source_file = get_data_file_path(&browser, &profile_dir, data_type.clone())
         .ok_or_else(|| format!("{:?} {:?} data file not supported or found", browser, data_type))?;

    if source_file.exists() {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let extension = source_file.extension().unwrap_or_default().to_str().unwrap_or("");
        let out_filename = format!("{:?}_{:?}-{}.{}",
             browser,
             data_type,
             ts,
             if extension.is_empty() { "bak" } else { extension }
         ).to_lowercase().replace("(", "_").replace(")", ""); // Sanitize filename

        let out_path = PathBuf::from(&out_filename);
        fs::copy(&source_file, &out_path)?;

        // Special case for Firefox passwords: also copy key4.db if it exists
        if browser == BrowserType::Firefox && data_type == BrowserDataType::Passwords {
            let key_db_path = profile_dir.join("key4.db");
            if key_db_path.exists() {
                let key_out_filename = format!("{:?}_key4db-{}.bak", browser, ts).to_lowercase();
                let key_out_path = PathBuf::from(&key_out_filename);
                fs::copy(&key_db_path, &key_out_path)?;
                println!("{} Also exported key database to {}", "✓".green(), key_out_path.display());
            }
        }

        let message = format!("Exported {:?} {:?} to {}", browser, data_type, out_path.display());
        println!("{} {}", "✓".green(), message);
        Ok(BrowserOpResult { success: true, message, export_path: Some(out_path) })
    } else {
        let message = format!("{:?} {:?} file not found at {}", browser, data_type, source_file.display());
        Err(message.into())
    }
}
