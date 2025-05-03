use anyhow::{Result, Context};
use colored::*;
use reqwest::{Client, header};
use serde_json::Value;
use std::path::Path;
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use futures::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::AsyncWriteExt;
use tokio::fs::File;
use std::time::Duration;
use regex::Regex;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;

lazy_static! {
    static ref USER_AGENTS: Vec<&'static str> = vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.1.1 Safari/605.1.15",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:89.0) Gecko/20100101 Firefox/89.0",
    ];
}

/// Search options for image downloads
#[derive(Debug, Clone)]
pub struct ImageSearchOptions {
    pub query: String,
    pub count: usize,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub color: Option<String>,
    pub safe_search: bool,
    pub concurrent_downloads: usize,
}

impl Default for ImageSearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            count: 10,
            width: None,
            height: None,
            min_width: Some(800),
            min_height: Some(600),
            max_width: None,
            max_height: None,
            color: None,
            safe_search: true,
            concurrent_downloads: 5,
        }
    }
}

/// Represents an image found during search
#[derive(Debug, Clone)]
pub struct ImageResult {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub source: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
}

/// Create a reqwest client with random user agent
fn create_client() -> Result<Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"),
    );
    
    // Pick a random user agent to avoid detection
    let user_agent = USER_AGENTS.choose(&mut rand::thread_rng()).unwrap_or(&USER_AGENTS[0]);
    
    let client = Client::builder()
        .default_headers(headers)
        .user_agent(*user_agent)
        .timeout(Duration::from_secs(30))
        .build()?;
    
    Ok(client)
}

/// Search for images using various APIs and web sources
pub async fn search_images(options: &ImageSearchOptions) -> Result<Vec<ImageResult>> {
    println!("{} {}", "Searching for images:".cyan().bold(), options.query);
    
    // Split search terms by comma if present
    let search_terms: Vec<String> = options
        .query
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    println!("{} {} separate search terms", "Found".green(), search_terms.len());
    
    let mut all_results = Vec::new();
    
    // Search for each term separately
    for (index, term) in search_terms.iter().enumerate() {
        println!("\n{} {} of {}: '{}'", "Searching term".cyan(), index + 1, search_terms.len(), term);
        
        // Create a modified options with just this search term
        let mut term_options = options.clone();
        term_options.query = term.clone();
        
        // Try multiple sources and combine results for this term
        let mut term_results = Vec::new();
        
        // Try Pixabay API first (free API with generous limits)
        match search_pixabay(&term_options).await {
            Ok(images) => {
                println!("{} {} images from Pixabay for '{}'", "Found".green(), images.len(), term);
                term_results.extend(images);
            },
            Err(e) => {
                println!("{} from Pixabay: {}", "Search error".yellow(), e);
            }
        }
        
        // If we need more results, try Unsplash
        if term_results.len() < term_options.count {
            match search_unsplash(&term_options).await {
                Ok(images) => {
                    println!("{} {} additional images from Unsplash for '{}'", "Found".green(), images.len(), term);
                    term_results.extend(images);
                },
                Err(e) => {
                    println!("{} from Unsplash: {}", "Search error".yellow(), e);
                }
            }
        }
        
        // If we still need more or both APIs failed, try web scraping
        if term_results.len() < term_options.count {
            match search_bing_images(&term_options).await {
                Ok(images) => {
                    println!("{} {} additional images from web search for '{}'", "Found".green(), images.len(), term);
                    term_results.extend(images);
                },
                Err(e) => {
                    println!("{} from web search: {}", "Search error".yellow(), e);
                }
            }
        }
        
        // Deduplicate by URL for this term
        let mut unique_urls = std::collections::HashSet::new();
        term_results.retain(|img| unique_urls.insert(img.url.clone()));
        
        // Limit to the requested count per term (divided by number of terms)
        let per_term_count = (options.count / search_terms.len()).max(1);
        if term_results.len() > per_term_count {
            term_results.truncate(per_term_count);
        }
        
        println!("{} {} unique images for '{}' term", "Found".green(), term_results.len(), term);
        
        // Add to our overall results
        all_results.extend(term_results);
    }
    
    // Deduplicate again across all terms
    let mut unique_urls = std::collections::HashSet::new();
    all_results.retain(|img| unique_urls.insert(img.url.clone()));
    
    // Limit to the requested count
    if all_results.len() > options.count {
        all_results.truncate(options.count);
    }
    
    println!("\n{} {} unique images in total across all search terms", "Found".green(), all_results.len());
    
    Ok(all_results)
}

/// Search Pixabay API for images
async fn search_pixabay(options: &ImageSearchOptions) -> Result<Vec<ImageResult>> {
    // Pixabay API key - this is a free API key with rate limits
    // In production, this should be stored in an environment variable or config file
    let api_key = "30908129-8fb1c0b20e978aea862cfc42c";
    
    let client = create_client()?;
    
    // Clean the query - remove commas and replace spaces with +
    let clean_query = options.query
        .replace(',', " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("+");
    
    let mut params = vec![
        ("key", api_key.to_string()),
        ("q", clean_query),
        ("image_type", "photo".to_string()),
        ("per_page", options.count.to_string()),
    ];
    
    if options.safe_search {
        params.push(("safesearch", "true".to_string()));
    }
    
    if let Some(min_width) = options.min_width {
        params.push(("min_width", min_width.to_string()));
    }
    
    if let Some(min_height) = options.min_height {
        params.push(("min_height", min_height.to_string()));
    }
    
    let response = client.get("https://pixabay.com/api/")
        .query(&params)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Pixabay API returned error: {}", response.status()));
    }
    
    let json: Value = response.json().await?;
    
    let mut results = Vec::new();
    
    if let Some(hits) = json.get("hits").and_then(|h| h.as_array()) {
        for hit in hits {
            if let (Some(url), Some(width), Some(height)) = (
                hit.get("largeImageURL").and_then(|u| u.as_str()),
                hit.get("imageWidth").and_then(|w| w.as_u64()).map(|w| w as u32),
                hit.get("imageHeight").and_then(|h| h.as_u64()).map(|h| h as u32),
            ) {
                let description = hit.get("tags").and_then(|t| t.as_str()).map(|s| s.to_string());
                let thumbnail = hit.get("previewURL").and_then(|u| u.as_str()).map(|s| s.to_string());
                
                results.push(ImageResult {
                    url: url.to_string(),
                    width,
                    height,
                    source: "Pixabay".to_string(),
                    description,
                    thumbnail_url: thumbnail,
                });
            }
        }
    }
    
    Ok(results)
}

/// Search Unsplash API for images
async fn search_unsplash(options: &ImageSearchOptions) -> Result<Vec<ImageResult>> {
    // Clean the query - remove commas and replace spaces with +
    let clean_query = options.query
        .replace(',', " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");
    
    // Demo API key that might be rate-limited, use a more reliable API option
    // Use a different free API key for Unsplash
    let access_key = "4DO3rlZ4NbLqvki5PWOeQMVVYAK-iKcGIY07us9tSCM";
    
    let client = create_client()?;
    
    let mut params = vec![
        ("query", clean_query),
        ("per_page", options.count.to_string()),
    ];
    
    if let Some(color) = &options.color {
        params.push(("color", color.clone()));
    }
    
    let response = client.get("https://api.unsplash.com/search/photos")
        .header("Authorization", format!("Client-ID {}", access_key))
        .query(&params)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Unsplash API returned error: {}", response.status()));
    }
    
    let json: Value = response.json().await?;
    
    let mut results = Vec::new();
    
    if let Some(results_array) = json.get("results").and_then(|r| r.as_array()) {
        for result in results_array {
            if let Some(urls) = result.get("urls") {
                let url = urls.get("full").and_then(|u| u.as_str()).unwrap_or_else(|| {
                    urls.get("regular").and_then(|u| u.as_str()).unwrap_or("")
                });
                
                if url.is_empty() {
                    continue;
                }
                
                let thumbnail = urls.get("thumb").and_then(|u| u.as_str()).map(|s| s.to_string());
                
                let width = result.get("width").and_then(|w| w.as_u64()).unwrap_or(0) as u32;
                let height = result.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32;
                
                let description = result.get("description")
                    .and_then(|d| d.as_str())
                    .or_else(|| result.get("alt_description").and_then(|d| d.as_str()))
                    .map(|s| s.to_string());
                
                results.push(ImageResult {
                    url: url.to_string(),
                    width,
                    height,
                    source: "Unsplash".to_string(),
                    description,
                    thumbnail_url: thumbnail,
                });
            }
        }
    }
    
    Ok(results)
}

/// Search for images using web scraping (Bing Images)
async fn search_bing_images(options: &ImageSearchOptions) -> Result<Vec<ImageResult>> {
    let client = create_client()?;
    
    // Clean the query - remove commas and replace spaces with +
    let clean_query = options.query
        .replace(',', " ")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("+");
    
    let url = format!("https://www.bing.com/images/search?q={}&form=HDRSC2&first=1", clean_query);
    
    let response = client.get(&url)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Bing Images returned error status: {}", response.status()));
    }
    
    let html = response.text().await?;
    
    // Extract image data from the HTML
    // Bing stores image data in a JSON-like structure within script tags
    let mut results = Vec::new();
    
    // Look for different patterns in the HTML to find images
    lazy_static! {
        // Try to match the typical iusc structure
        static ref IMAGE_REGEX: Regex = Regex::new(r#"\{"murl":"([^"]+)","[^}]+"turl":"([^"]+)"[^}]+"t":"([^"]*)"[^}]+"w":(\d+),"h":(\d+)"#).unwrap();
        
        // Add an alternative regex pattern to find more images
        static ref ALT_IMAGE_REGEX: Regex = Regex::new(r#"<img[^>]+src="([^"]+)"[^>]+alt="([^"]+)"[^>]+"#).unwrap();
    }
    
    // Try the first regex pattern
    for cap in IMAGE_REGEX.captures_iter(&html) {
        let url = &cap[1];
        let thumbnail = &cap[2];
        let alt_text = &cap[3];
        let width: u32 = cap[4].parse().unwrap_or(0);
        let height: u32 = cap[5].parse().unwrap_or(0);
        
        // Skip non-image URLs or tiny thumbnails
        if !url.ends_with(".jpg") && !url.ends_with(".jpeg") && !url.ends_with(".png") {
            continue;
        }
        
        // Apply filtering based on dimensions
        if let Some(min_width) = options.min_width {
            if width < min_width {
                continue;
            }
        }
        
        if let Some(min_height) = options.min_height {
            if height < min_height {
                continue;
            }
        }
        
        if let Some(max_width) = options.max_width {
            if width > max_width {
                continue;
            }
        }
        
        if let Some(max_height) = options.max_height {
            if height > max_height {
                continue;
            }
        }
        
        results.push(ImageResult {
            url: url.to_string(),
            width,
            height,
            source: "Bing".to_string(),
            description: Some(alt_text.to_string()),
            thumbnail_url: Some(thumbnail.to_string()),
        });
        
        if results.len() >= options.count {
            break;
        }
    }
    
    // If we didn't find enough images, try the alternate pattern
    if results.len() < options.count {
        for cap in ALT_IMAGE_REGEX.captures_iter(&html) {
            let url = &cap[1];
            let alt_text = &cap[2];
            
            // Skip non-image URLs or data URLs
            if !url.starts_with("http") || url.starts_with("data:") {
                continue;
            }
            
            results.push(ImageResult {
                url: url.to_string(),
                width: 800, // Default width
                height: 600, // Default height
                source: "Bing".to_string(),
                description: Some(alt_text.to_string()),
                thumbnail_url: Some(url.to_string()),
            });
            
            if results.len() >= options.count {
                break;
            }
        }
    }
    
    // If still no results, try a different approach - look for "src" attributes in img tags
    if results.is_empty() {
        let img_regex = Regex::new(r#"<img[^>]+src="([^"]+)"[^>]*>"#).unwrap();
        for cap in img_regex.captures_iter(&html) {
            let url = &cap[1];
            
            // Skip tiny images, data URLs, or non-URLs
            if !url.starts_with("http") || url.starts_with("data:") || url.contains("&w=40") {
                continue;
            }
            
            results.push(ImageResult {
                url: url.to_string(),
                width: 800, // Default width 
                height: 600, // Default height
                source: "Bing".to_string(),
                description: None,
                thumbnail_url: Some(url.to_string()),
            });
            
            if results.len() >= options.count {
                break;
            }
        }
    }
    
    Ok(results)
}

/// Download a batch of images to a directory
pub async fn download_images(
    images: &[ImageResult],
    output_dir: &Path,
    concurrent_downloads: usize,
) -> Result<()> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;
    
    println!("{} {} images to {}", "Downloading".cyan().bold(), images.len(), output_dir.display());
    
    // Setup for concurrent downloads
    let semaphore = Arc::new(Semaphore::new(concurrent_downloads));
    
    // Setup progress display
    let mp = MultiProgress::new();
    let main_pb = mp.add(ProgressBar::new(images.len() as u64));
    main_pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} images ({eta})")
        .unwrap()
        .progress_chars("#>-"));
    
    // Clone client for all downloads
    let client = create_client()?;
    
    // Create download tasks
    let download_tasks = images.iter().enumerate().map(|(i, image)| {
        // Clone what we need for the task
        let semaphore = Arc::clone(&semaphore);
        let client = client.clone();
        let url = image.url.clone();
        let _source = image.source.clone();
        let output_dir = output_dir.to_path_buf();
        let main_pb = main_pb.clone();
        
        // Create a progress bar for this download
        let pb = mp.add(ProgressBar::new(0));
        pb.set_style(ProgressStyle::default_bar()
            .template(&format!("{{spinner:.green}} Image {} [{{bar:30.cyan/blue}}] {{bytes}}/{{total_bytes}} ({{eta}})", i+1))
            .unwrap()
            .progress_chars("#>-"));
        
        async move {
            // Acquire permit from semaphore
            let _permit = semaphore.acquire().await.unwrap();
            
            // Extract filename from URL and sanitize it
            let filename = extract_filename_from_url(&url, i).unwrap_or_else(|| {
                format!("image_{:03}.jpg", i+1)
            });
            
            let output_path = output_dir.join(&filename);
            
            // Download the file
            let success = match download_single_image(&client, &url, &output_path, pb.clone()).await {
                Ok(()) => true,
                Err(e) => {
                    println!("{} {}: {}", "Failed to download".red(), filename, e);
                    false
                }
            };
            
            // Update main progress
            main_pb.inc(1);
            pb.finish_and_clear();
            
            (success, output_path)
        }
    });
    
    // Start progress bars in separate thread
    let mp_handle = tokio::task::spawn_blocking(move || {
        // Keep mp alive
    });
    
    // Wait for all downloads to complete
    let results = futures::future::join_all(download_tasks).await;
    
    // Wait for progress display to finish
    let _ = mp_handle.await;
    
    // Count successes
    let successful = results.iter().filter(|(success, _)| *success).count();
    
    main_pb.finish_with_message(format!("{}/{} images downloaded", successful, images.len()).green().to_string());
    
    if successful > 0 {
        println!("{} {} {} {}", "Successfully downloaded".green().bold(), successful, "images to", output_dir.display());
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to download any images"))
    }
}

/// Download a single image with progress
async fn download_single_image(
    client: &Client,
    url: &str,
    output_path: &Path,
    progress_bar: ProgressBar,
) -> Result<()> {
    // Make the request
    let response = client.get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download image from {}", url))?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to download image: HTTP status {}", response.status()));
    }
    
    // Get content length for progress
    let content_length = response.content_length().unwrap_or(0);
    progress_bar.set_length(content_length);
    
    // Open file for writing
    let mut file = File::create(output_path).await?;
    
    // Stream the download with progress updates
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
        
        downloaded += chunk.len() as u64;
        progress_bar.set_position(downloaded);
    }
    
    // Ensure file is fully written
    file.flush().await?;
    
    progress_bar.finish_with_message("Complete".green().to_string());
    Ok(())
}

/// Extract a filename from a URL
fn extract_filename_from_url(url: &str, fallback_index: usize) -> Option<String> {
    // Try to get the path part of the URL
    let url_path = url.split('?').next()?;
    
    // Get the last segment of the path
    let filename = url_path.split('/').last()?;
    
    // Check if it has a file extension
    if filename.contains('.') {
        // Sanitize the filename
        let sanitized = sanitize_filename(filename);
        Some(sanitized)
    } else {
        // No valid filename found, use fallback
        Some(format!("image_{:03}.jpg", fallback_index + 1))
    }
}

/// Sanitize a filename to be safe for filesystem
fn sanitize_filename(filename: &str) -> String {
    // Remove any characters that are not safe for filenames
    let sanitized: String = filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    
    // Ensure the filename isn't too long
    if sanitized.len() > 100 {
        sanitized[0..100].to_string()
    } else {
        sanitized
    }
}

/// Display information about an image
pub fn display_image_info(image: &ImageResult) {
    println!("{}: {}", "URL".green(), image.url);
    println!("{}: {}x{}", "Dimensions".green(), image.width, image.height);
    println!("{}: {}", "Source".green(), image.source);
    
    if let Some(desc) = &image.description {
        println!("{}: {}", "Description".green(), desc);
    }
    
    if let Some(thumb) = &image.thumbnail_url {
        println!("{}: {}", "Thumbnail".green(), thumb);
    }
} 