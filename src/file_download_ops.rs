use anyhow::Result;
use colored::*;
use futures::stream::StreamExt;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use reqwest::{Client, StatusCode};
use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;
use std::time::Duration;

/// Downloads a file from a URL, with support for retries, resuming, and parallel downloads
pub async fn download_file(
    url: &str, 
    output_path: &Path, 
    retries: usize,
    resume: bool,
    parallel: usize
) -> Result<()> {
    println!("{} {}", "Downloading:".cyan().bold(), url);
    println!("{} {}", "Output file:".cyan().bold(), output_path.display());
    
    // Create a client with a timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    // First, perform a HEAD request to get the file size and check if the server supports range requests
    let head_resp = client.head(url).send().await?;
    
    if !head_resp.status().is_success() {
        return Err(anyhow::anyhow!("Failed to fetch file information: HTTP status {}", head_resp.status()));
    }
    
    let supports_range = head_resp.headers().get("accept-ranges")
        .map(|v| v.to_str().unwrap_or("").contains("bytes"))
        .unwrap_or(false);
    
    let total_size = head_resp.headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse::<u64>().ok())
        .unwrap_or(0);
    
    if total_size == 0 {
        println!("{}", "Warning: Could not determine file size. Progress reporting may be inaccurate.".yellow());
    }
    
    if parallel > 1 && (!supports_range || total_size == 0) {
        println!("{}", "Warning: The server doesn't support range requests or file size is unknown. Parallel download disabled.".yellow());
        return download_single(url, output_path, retries, resume, total_size, &client).await;
    }
    
    if parallel > 1 {
        download_parallel(url, output_path, retries, resume, total_size, parallel, &client).await
    } else {
        download_single(url, output_path, retries, resume, total_size, &client).await
    }
}

/// Performs a single-threaded download with retry and resume support
async fn download_single(
    url: &str,
    output_path: &Path,
    retries: usize,
    can_resume: bool,
    total_size: u64,
    client: &Client
) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let mut file_size: u64 = 0;
    let mut file: File;
    
    // Check if we can resume a previous download
    if can_resume && output_path.exists() {
        file_size = std::fs::metadata(output_path)?.len();
        
        if file_size >= total_size && total_size > 0 {
            println!("{}", "File is already fully downloaded.".green());
            return Ok(());
        }
        
        println!("{} {} of {} bytes", "Resuming from:".cyan(), file_size, total_size);
        file = OpenOptions::new().write(true).append(true).open(output_path)?;
    } else {
        // Start a new download
        file = File::create(output_path)?;
    }
    
    // Set up the progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    pb.set_position(file_size);
    
    let mut retry_count = 0;
    let mut success = false;
    
    while retry_count <= retries && !success {
        if retry_count > 0 {
            let wait_time = std::cmp::min(2u64.pow(retry_count as u32), 60);
            println!("{} {} seconds before retry {}/{}", "Waiting".yellow(), wait_time, retry_count, retries);
            tokio::time::sleep(Duration::from_secs(wait_time)).await;
        }
        
        let mut request = client.get(url);
        
        // Add range header if resuming
        if file_size > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={}-", file_size));
        }
        
        match request.send().await {
            Ok(resp) => {
                if !resp.status().is_success() && resp.status() != StatusCode::PARTIAL_CONTENT {
                    println!("{} {}: {}", "Error:".red(), "HTTP error", resp.status());
                    retry_count += 1;
                    continue;
                }
                
                let mut stream = resp.bytes_stream();
                
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            file.write_all(&chunk)?;
                            pb.inc(chunk.len() as u64);
                        },
                        Err(e) => {
                            println!("{} {}: {}", "Error:".red(), "Failed to download chunk", e);
                            retry_count += 1;
                            // If we're resuming, get the new file size
                            if can_resume {
                                file_size = file.metadata()?.len();
                                file.flush()?;
                            }
                            break;
                        }
                    }
                }
                
                success = true;
            },
            Err(e) => {
                println!("{} {}: {}", "Error:".red(), "Failed to send request", e);
                retry_count += 1;
            }
        }
    }
    
    pb.finish_with_message(if success { "Download complete".green().to_string() } else { "Download failed".red().to_string() });
    
    if !success {
        return Err(anyhow::anyhow!("Failed to download file after {} retries", retries));
    }
    
    Ok(())
}

/// Performs a parallel download with multiple connections
async fn download_parallel(
    url: &str,
    output_path: &Path,
    retries: usize,
    can_resume: bool,
    total_size: u64,
    parallel: usize,
    client: &Client
) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Initialize the file with zeros to pre-allocate space
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(!can_resume)
        .open(output_path)?;
    
    if !can_resume || !output_path.exists() {
        file.set_len(total_size)?;
    }
    
    // Calculate chunk sizes
    let chunk_size = total_size / parallel as u64;
    let mut download_tasks = Vec::new();
    let client = Arc::new(client.clone());
    
    // Set up a multi-progress bar
    let multi_progress = MultiProgress::new();
    let main_pb = multi_progress.add(ProgressBar::new(total_size));
    main_pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    // Limit concurrent downloads with a semaphore
    let semaphore = Arc::new(Semaphore::new(parallel));
    
    // Spawn a separate task to run the progress bars
    let _mp_handle = task::spawn_blocking(move || {
        // Remove the call to join() since it doesn't exist
        // Just keep the multi_progress alive in this thread
        // multi_progress will be dropped when this task completes
    });
    
    // Create one task per chunk
    for i in 0..parallel {
        let start = i as u64 * chunk_size;
        let mut end = min((i as u64 + 1) * chunk_size - 1, total_size - 1);
        if i == parallel - 1 {
            end = total_size - 1; // Make sure the last chunk gets any remaining bytes
        }
        
        // Skip already completed chunks (for resume)
        let temp_path = get_temp_path(output_path, i);
        let mut current_pos = 0;
        
        if can_resume && temp_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&temp_path) {
                current_pos = metadata.len();
                if current_pos >= end - start + 1 {
                    // This chunk is already complete
                    println!("{} {}", "Chunk".green(), i + 1);
                    continue;
                }
            }
        }
        
        let client_clone = client.clone();
        let url = url.to_string();
        let semaphore_clone = semaphore.clone();
        let output_path = output_path.to_path_buf();
        let pb = multi_progress.add(ProgressBar::new(end - start + 1));
        
        pb.set_style(ProgressStyle::default_bar()
            .template(&format!("{{spinner:.green}} Chunk {} [{{bar:20.cyan/blue}}] {{bytes}}/{{total_bytes}}", i + 1))
            .unwrap()
            .progress_chars("#>-"));
        
        pb.set_position(current_pos);
        
        // Download a single chunk
        let task = task::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            
            let chunk_result = download_chunk(
                &url, 
                &output_path, 
                start, 
                end, 
                retries,
                can_resume,
                current_pos,
                pb.clone(),
                i,
                &client_clone
            ).await;
            
            pb.finish_and_clear();
            chunk_result
        });
        
        download_tasks.push(task);
    }
    
    // Wait for all downloads to complete
    let mut success = true;
    for task in download_tasks {
        match task.await {
            Ok(result) => {
                if let Err(e) = result {
                    println!("{} {}", "Chunk error:".red(), e);
                    success = false;
                } else {
                    main_pb.inc(chunk_size);
                }
            },
            Err(e) => {
                println!("{} {}", "Task error:".red(), e);
                success = false;
            }
        }
    }
    
    main_pb.finish_with_message(if success { "Download complete".green().to_string() } else { "Download failed".red().to_string() });
    
    // If the download was successful, combine all chunks into the final file
    if success {
        let mut output_file = OpenOptions::new()
            .write(true)
            .open(output_path)?;
        
        for i in 0..parallel {
            let temp_path = get_temp_path(output_path, i);
            if temp_path.exists() {
                let start = i as u64 * chunk_size;
                let mut temp_file = File::open(&temp_path)?;
                let temp_size = temp_file.metadata()?.len();
                
                let mut buffer = vec![0u8; temp_size as usize];
                temp_file.read_exact(&mut buffer)?;
                
                output_file.seek(SeekFrom::Start(start))?;
                output_file.write_all(&buffer)?;
                
                // Remove the temporary file
                std::fs::remove_file(temp_path)?;
            }
        }
    }
    
    if !success {
        return Err(anyhow::anyhow!("Failed to download one or more chunks"));
    }
    
    Ok(())
}

/// Downloads a single chunk of the file
async fn download_chunk(
    url: &str,
    output_path: &Path,
    start: u64,
    end: u64,
    retries: usize,
    can_resume: bool,
    current_pos: u64,
    pb: ProgressBar,
    chunk_idx: usize,
    client: &Client
) -> Result<()> {
    let temp_path = get_temp_path(output_path, chunk_idx);
    
    // Create or open temporary file for this chunk
    let mut file = if can_resume && temp_path.exists() && current_pos > 0 {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&temp_path)?;
        file
    } else {
        let file = File::create(&temp_path)?;
        file
    };
    
    let mut retry_count = 0;
    let mut success = false;
    let actual_start = start + current_pos;
    
    // Don't retry if we've completed the chunk
    if actual_start > end {
        return Ok(());
    }
    
    while retry_count <= retries && !success {
        if retry_count > 0 {
            let wait_time = std::cmp::min(2u64.pow(retry_count as u32), 60);
            tokio::time::sleep(Duration::from_secs(wait_time)).await;
        }
        
        let range = format!("bytes={}-{}", actual_start, end);
        
        match client.get(url)
            .header(reqwest::header::RANGE, range)
            .send()
            .await {
                Ok(resp) => {
                    if resp.status() != StatusCode::PARTIAL_CONTENT && resp.status() != StatusCode::OK {
                        retry_count += 1;
                        continue;
                    }
                    
                    let mut stream = resp.bytes_stream();
                    
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                file.write_all(&chunk)?;
                                pb.inc(chunk.len() as u64);
                            },
                            Err(_) => {
                                retry_count += 1;
                                file.flush()?;
                                break;
                            }
                        }
                    }
                    
                    success = true;
                },
                Err(_) => {
                    retry_count += 1;
                }
            }
    }
    
    if !success {
        return Err(anyhow::anyhow!("Failed to download chunk {} after {} retries", chunk_idx, retries));
    }
    
    Ok(())
}

// Helper function to get the temporary path for a chunk
fn get_temp_path(output_path: &Path, chunk_idx: usize) -> PathBuf {
    let filename = output_path.file_name().unwrap().to_str().unwrap();
    let parent = output_path.parent().unwrap_or_else(|| Path::new(""));
    parent.join(format!("{}.part{}", filename, chunk_idx))
}

// Helper to format bytes to human-readable form
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