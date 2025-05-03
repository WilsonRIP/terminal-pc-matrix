use anyhow::Result;
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::{Command, Stdio};
use tokio::task;
use std::io::{BufRead, BufReader};
use regex::Regex;
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::collections::HashMap;
use std::time::Duration;
use lazy_static::lazy_static;

lazy_static! {
    static ref PROGRESS_REGEX: Regex = Regex::new(r"\[download\]\s+(\d+\.\d+)%").unwrap();
    static ref PLAYLIST_REGEX: Regex = Regex::new(r"\[download\] Downloading item (\d+) of (\d+)").unwrap();
}

// Default yt-dlp arguments that improve performance
const DEFAULT_ARGS: &[&str] = &[
    "--no-check-certificate",  // Skip HTTPS certificate validation (faster)
    "--no-call-home",          // Disable call home behavior
    "--no-warnings",           // Suppress warnings, which saves processing time
    "--buffer-size", "16M",    // Use a larger buffer for faster downloads
    "--socket-timeout", "15",  // Faster timeout if connections hang
    "--no-playlist-reverse",   // Don't waste time reversing playlists
    "--concurrent-fragments", "5", // Download multiple fragments concurrently
];

/// Video quality options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoQuality {
    Best,
    HD1080,
    HD720,
    SD480,
    Lowest,
    AudioOnly,
}

impl VideoQuality {
    pub fn to_ytdlp_arg(&self) -> &'static str {
        match self {
            VideoQuality::Best => "bestvideo+bestaudio/best",
            VideoQuality::HD1080 => "bestvideo[height<=1080]+bestaudio/best[height<=1080]",
            VideoQuality::HD720 => "bestvideo[height<=720]+bestaudio/best[height<=720]",
            VideoQuality::SD480 => "bestvideo[height<=480]+bestaudio/best[height<=480]",
            VideoQuality::Lowest => "worstvideo+worstaudio/worst",
            VideoQuality::AudioOnly => "bestaudio/best",
        }
    }
    
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "best" => Some(VideoQuality::Best),
            "1080p" | "1080" | "hd1080" => Some(VideoQuality::HD1080),
            "720p" | "720" | "hd720" => Some(VideoQuality::HD720),
            "480p" | "480" | "sd480" => Some(VideoQuality::SD480),
            "lowest" => Some(VideoQuality::Lowest),
            "audio" | "audioonly" | "audio-only" => Some(VideoQuality::AudioOnly),
            _ => None,
        }
    }
    
    pub fn display_options() -> String {
        "Available qualities: best, 1080p, 720p, 480p, lowest, audio-only".to_string()
    }
}

/// Download options struct for better configuration
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub quality: VideoQuality,
    pub audio_only: bool,
    pub max_rate: Option<String>,    // Bandwidth limit (e.g., "1M")
    pub concurrent_downloads: usize, // Number of parallel playlist items
    pub cookies_file: Option<String>, // Optional cookies file for auth
    pub subtitles: bool,             // Download subtitles
    pub force_ipv4: bool,            // Force IPv4 (sometimes faster)
    pub proxy: Option<String>,       // Optional proxy URL
    pub retries: usize,              // Number of retries
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            quality: VideoQuality::Best,
            audio_only: false,
            max_rate: None,
            concurrent_downloads: 3,
            cookies_file: None,
            subtitles: false,
            force_ipv4: true,
            proxy: None,
            retries: 10,
        }
    }
}

/// Check if yt-dlp is installed on the system
pub async fn check_ytdlp_installed() -> bool {
    match Command::new("yt-dlp")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status() {
            Ok(_) => true,
            Err(_) => false,
        }
}

/// Download a video from a URL with specified options
pub async fn download_video(
    url: &str,
    output_dir: &Path,
    quality: VideoQuality,
    audio_only: bool,
) -> Result<()> {
    let options = DownloadOptions {
        quality,
        audio_only,
        ..Default::default()
    };
    
    download_video_with_options(url, output_dir, &options).await
}

/// Download a video with detailed options
pub async fn download_video_with_options(
    url: &str,
    output_dir: &Path,
    options: &DownloadOptions,
) -> Result<()> {
    // Check if yt-dlp is installed
    if !check_ytdlp_installed().await {
        return Err(anyhow::anyhow!("yt-dlp is not installed. Please install it first: https://github.com/yt-dlp/yt-dlp#installation"));
    }
    
    println!("{} {}", "Downloading video from:".cyan().bold(), url);
    println!("{} {}", "Output directory:".cyan().bold(), output_dir.display());
    println!("{} {:?}", "Selected quality:".cyan().bold(), options.quality);
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;
    
    // Check if URL is a playlist
    if is_playlist(url).await? {
        return download_playlist(url, output_dir, options).await;
    }
    
    // For single video
    let format = if options.audio_only || options.quality == VideoQuality::AudioOnly {
        "bestaudio/best"
    } else {
        options.quality.to_ytdlp_arg()
    };
    
    // Setup file extension
    let _ext = if options.audio_only || options.quality == VideoQuality::AudioOnly {
        "mp3"
    } else {
        "mp4"
    };
    
    // Setup output template
    let output_template = output_dir.join("%(title)s.%(ext)s");
    
    // Create progress bar
    let pb = ProgressBar::new(100);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% ({eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    // Build command with optimized arguments
    let mut cmd = Command::new("yt-dlp");
    cmd.arg(url)
        .arg("-f").arg(format)
        .arg("-o").arg(output_template.to_string_lossy().to_string())
        .args(DEFAULT_ARGS)
        .arg("--retries").arg(options.retries.to_string());
    
    // Add audio conversion if audio only
    if options.audio_only || options.quality == VideoQuality::AudioOnly {
        cmd.arg("-x")
           .arg("--audio-format").arg("mp3");
    }
    
    // Add optional rate limiting
    if let Some(rate) = &options.max_rate {
        cmd.arg("--limit-rate").arg(rate);
    }
    
    // Add subtitles if requested
    if options.subtitles {
        cmd.arg("--write-auto-subs").arg("--sub-langs").arg("en.*");
    }
    
    // Force IPv4 if requested (can be faster)
    if options.force_ipv4 {
        cmd.arg("--force-ipv4");
    }
    
    // Add proxy if specified
    if let Some(proxy) = &options.proxy {
        cmd.arg("--proxy").arg(proxy);
    }
    
    // Add cookies file if specified (for authenticated downloads)
    if let Some(cookies) = &options.cookies_file {
        cmd.arg("--cookies").arg(cookies);
    }
    
    // Execute command with capture progress
    let mut process = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    // Setup progress tracking
    let stderr = process.stderr.take().expect("Failed to take stderr");
    let reader = BufReader::new(stderr);
    
    // Track progress in a separate task
    let pb_clone = pb.clone();
    let progress_task = task::spawn_blocking(move || {
        for line in reader.lines() {
            if let Ok(line) = line {
                if let Some(caps) = PROGRESS_REGEX.captures(&line) {
                    if let Some(percent_match) = caps.get(1) {
                        if let Ok(percent) = percent_match.as_str().parse::<f64>() {
                            pb_clone.set_position((percent * 100.0) as u64);
                        }
                    }
                }
                // Print the line if it contains important info (filter out progress lines)
                if !line.contains("[download]") || line.contains("Destination") || line.contains("error") {
                    println!("{}", line);
                }
            }
        }
    });
    
    // Wait for the command to complete
    let status = process.wait()?;
    
    // Wait for progress tracking to complete
    let _ = progress_task.await;
    
    // Check if command was successful
    if !status.success() {
        pb.finish_with_message("Download failed".red().to_string());
        return Err(anyhow::anyhow!("Failed to download video: yt-dlp exited with status {}", status));
    }
    
    pb.finish_with_message("Download complete".green().to_string());
    println!("{} {}", "Video downloaded to:".green().bold(), output_dir.display());
    
    Ok(())
}

/// Check if a URL is a playlist
async fn is_playlist(url: &str) -> Result<bool> {
    let output = Command::new("yt-dlp")
        .arg("--flat-playlist")
        .arg("--dump-json")
        .arg(url)
        .output()?;
    
    if !output.status.success() {
        return Ok(false);
    }
    
    // Count the number of JSON objects (each line is one video)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().count();
    
    Ok(count > 1)
}

/// Download a playlist with parallel processing
async fn download_playlist(
    url: &str,
    output_dir: &Path,
    options: &DownloadOptions,
) -> Result<()> {
    println!("{}", "Playlist detected. Getting video list...".cyan());
    
    // First, get the list of videos in the playlist
    let video_ids = get_playlist_video_ids(url).await?;
    let total_videos = video_ids.len();
    
    println!("{} {} videos", "Found".green(), total_videos);
    
    if total_videos == 0 {
        return Err(anyhow::anyhow!("No videos found in playlist"));
    }
    
    // Set up a multi-progress display
    let mp = MultiProgress::new();
    let main_pb = mp.add(ProgressBar::new(total_videos as u64));
    main_pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.magenta/blue}] {pos}/{len} videos ({eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    // Set up a semaphore to limit concurrent downloads
    let max_concurrent = std::cmp::min(options.concurrent_downloads, total_videos);
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    
    println!("{} {} parallel downloads", "Using".cyan(), max_concurrent);
    
    // Generate the full playlist URL for each video
    let tasks = video_ids.into_iter().enumerate().map(|(i, id)| {
        let video_url = format!("https://www.youtube.com/watch?v={}", id);
        let output_dir = output_dir.to_path_buf();
        let options = options.clone();
        let sem_clone = semaphore.clone();
        let pb = mp.add(ProgressBar::new(100));
        pb.set_style(ProgressStyle::default_bar()
            .template(&format!("{{spinner:.green}} Video {} [{{bar:30.cyan/blue}}] {{percent}}% ({{eta}})", i+1))
            .unwrap()
            .progress_chars("#>-"));
        
        let main_pb_clone = main_pb.clone();
        
        async move {
            // Acquire permit from semaphore
            let _permit = sem_clone.acquire().await.unwrap();
            
            // Download individual video
            let format = if options.audio_only || options.quality == VideoQuality::AudioOnly {
                "bestaudio/best"
            } else {
                options.quality.to_ytdlp_arg()
            };
            
            // Prepare command for individual video
            let mut cmd = Command::new("yt-dlp");
            cmd.arg(&video_url)
                .arg("-f").arg(format)
                .arg("-o").arg(output_dir.join("%(title)s.%(ext)s").to_string_lossy().to_string())
                .args(DEFAULT_ARGS)
                .arg("--retries").arg(options.retries.to_string());
            
            if options.audio_only || options.quality == VideoQuality::AudioOnly {
                cmd.arg("-x").arg("--audio-format").arg("mp3");
            }
            
            // Add optional rate limiting
            if let Some(rate) = &options.max_rate {
                cmd.arg("--limit-rate").arg(rate);
            }
            
            // Force IPv4 if requested (can be faster)
            if options.force_ipv4 {
                cmd.arg("--force-ipv4");
            }
            
            let mut process = cmd
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            
            if let Some(stderr) = process.stderr.take() {
                let reader = BufReader::new(stderr);
                let pb_clone = pb.clone();
                
                task::spawn_blocking(move || {
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if let Some(caps) = PROGRESS_REGEX.captures(&line) {
                                if let Some(percent_match) = caps.get(1) {
                                    if let Ok(percent) = percent_match.as_str().parse::<f64>() {
                                        pb_clone.set_position((percent * 100.0) as u64);
                                    }
                                }
                            }
                        }
                    }
                });
            }
            
            let status = process.wait().unwrap();
            pb.finish_and_clear();
            
            main_pb_clone.inc(1);
            
            status.success()
        }
    });
    
    // Spawn the progress display in a separate thread
    let mp_handle = tokio::task::spawn_blocking(move || {
        // Keep mp alive until all progress bars are done
    });
    
    // Collect and process all download tasks
    let results: Vec<bool> = futures::future::join_all(tasks).await;
    
    // Wait for the progress display thread to finish
    let _ = mp_handle.await;
    
    // Count successful downloads
    let successes = results.iter().filter(|&&success| success).count();
    
    main_pb.finish_with_message(format!("{}/{} videos downloaded", successes, total_videos).green().to_string());
    
    if successes == total_videos {
        println!("{} {} {} {}", "Successfully downloaded".green().bold(), successes, "videos to", output_dir.display());
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to download {} videos", total_videos - successes))
    }
}

/// Get a list of video IDs from a playlist URL
async fn get_playlist_video_ids(url: &str) -> Result<Vec<String>> {
    let output = Command::new("yt-dlp")
        .arg("--flat-playlist")
        .arg("--print-to-file")
        .arg("%(id)s")
        .arg("-") // Print to stdout
        .arg(url)
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to get playlist information"));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let ids: Vec<String> = stdout.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    
    Ok(ids)
}

/// Get information about a video URL
pub async fn get_video_info(url: &str) -> Result<String> {
    // Check if yt-dlp is installed
    if !check_ytdlp_installed().await {
        return Err(anyhow::anyhow!("yt-dlp is not installed. Please install it first: https://github.com/yt-dlp/yt-dlp#installation"));
    }
    
    println!("{} {}", "Getting video information for:".cyan().bold(), url);
    
    // Use a timeout for potentially slow queries
    let output = tokio::time::timeout(
        Duration::from_secs(15), 
        tokio::process::Command::new("yt-dlp")
            .arg("--dump-json")
            .arg("--no-playlist") // Only get info for the main video, not the playlist
            .arg(url)
            .output()
    ).await??;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to get video info: {}", error));
    }
    
    let json = String::from_utf8_lossy(&output.stdout);
    
    // Format a simplified info output (not the raw JSON)
    let mut info = String::new();
    
    // Try to parse the JSON to extract useful fields
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
        if let Some(title) = parsed.get("title").and_then(|v| v.as_str()) {
            info.push_str(&format!("{}: {}\n", "Title".green(), title));
        }
        
        if let Some(uploader) = parsed.get("uploader").and_then(|v| v.as_str()) {
            info.push_str(&format!("{}: {}\n", "Uploader".green(), uploader));
        }
        
        if let Some(duration) = parsed.get("duration").and_then(|v| v.as_f64()) {
            let mins = (duration / 60.0).floor();
            let secs = duration % 60.0;
            info.push_str(&format!("{}: {:.0}:{:02.0}\n", "Duration".green(), mins, secs));
        }
        
        if let Some(view_count) = parsed.get("view_count").and_then(|v| v.as_u64()) {
            info.push_str(&format!("{}: {}\n", "View Count".green(), view_count));
        }
        
        if let Some(upload_date) = parsed.get("upload_date").and_then(|v| v.as_str()) {
            // Format YYYYMMDD as YYYY-MM-DD
            if upload_date.len() == 8 {
                let year = &upload_date[0..4];
                let month = &upload_date[4..6];
                let day = &upload_date[6..8];
                info.push_str(&format!("{}: {}-{}-{}\n", "Upload Date".green(), year, month, day));
            } else {
                info.push_str(&format!("{}: {}\n", "Upload Date".green(), upload_date));
            }
        }
        
        // Indicate if it's a playlist
        if parsed.get("playlist").is_some() {
            info.push_str(&format!("{}: {}\n", "Type".green(), "Playlist"));
            
            if let Some(playlist_count) = parsed.get("playlist_count").and_then(|v| v.as_u64()) {
                info.push_str(&format!("{}: {}\n", "Items".green(), playlist_count));
            }
        } else {
            info.push_str(&format!("{}: {}\n", "Type".green(), "Single Video"));
        }
        
        if let Some(description) = parsed.get("description").and_then(|v| v.as_str()) {
            // Truncate description if too long
            let desc = if description.len() > 200 {
                format!("{}...", &description[0..200])
            } else {
                description.to_string()
            };
            info.push_str(&format!("{}: {}\n", "Description".green(), desc));
        }
        
        // Add download size estimate
        if let Some(formats) = parsed.get("formats").and_then(|v| v.as_array()) {
            // Get largest format size as an estimate
            let mut max_filesize = 0;
            for format in formats {
                if let Some(filesize) = format.get("filesize").and_then(|v| v.as_u64()) {
                    if filesize > max_filesize {
                        max_filesize = filesize;
                    }
                }
            }
            
            if max_filesize > 0 {
                info.push_str(&format!("{}: {}\n", "Estimated Size".green(), format_bytes(max_filesize)));
            }
            
            // Format summary
            let mut format_counts = HashMap::new();
            for format in formats {
                if let Some(format_note) = format.get("format_note").and_then(|v| v.as_str()) {
                    format_counts.entry(format_note.to_string()).or_insert(0);
                    *format_counts.get_mut(format_note).unwrap() += 1;
                }
            }
            
            let format_summary: Vec<String> = format_counts.iter()
                .filter(|(k, _)| !k.is_empty())
                .map(|(k, v)| format!("{} ({})", k, v))
                .collect();
            
            if !format_summary.is_empty() {
                info.push_str(&format!("{}: {}\n", "Formats".green(), format_summary.join(", ")));
            }
        }
    } else {
        // If parsing fails, return a simpler message
        info = format!("Video information retrieved. Use download option to proceed.\n");
    }
    
    Ok(info)
}

/// Format bytes to human-readable size string
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
} 