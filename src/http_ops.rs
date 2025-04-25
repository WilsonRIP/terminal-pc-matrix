use colored::*;
use reqwest::{Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use serde_json;

pub async fn make_request(
    method_str: &str,
    url: &str,
    body: Option<&str>,
    headers_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    println!(
        "{} {} {}",
        "Running:".cyan(),
        method_str.to_uppercase().yellow(),
        url.cyan()
    );

    let client = Client::new();

    // Parse method
    let method = Method::from_str(&method_str.to_uppercase())
        .map_err(|_| format!("Invalid HTTP method: {}", method_str))?;

    // Build headers
    let mut headers = HeaderMap::new();
    for (key, value) in headers_map {
        match HeaderName::from_str(key) {
            Ok(header_name) => match HeaderValue::from_str(value) {
                Ok(header_value) => {
                    headers.insert(header_name, header_value);
                }
                Err(_) => {
                    eprintln!("{}: Invalid characters in header value for '{}'", "Warning".yellow(), key);
                }
            },
            Err(_) => {
                 eprintln!("{}: Invalid characters in header name '{}'", "Warning".yellow(), key);
            }
        }
    }

    // Build request
    let mut request_builder = client.request(method, url).headers(headers);
    if let Some(body_content) = body {
        request_builder = request_builder.body(body_content.to_string());
        println!("Body: {}", body_content.dimmed());
    }

    // Send request and measure time
    println!("{}", "Sending request...".dimmed());
    let start_time = std::time::Instant::now();
    let response = request_builder.send().await?;
    let duration = start_time.elapsed();

    println!("{}", "-".repeat(40).dimmed());

    // Print Response Status
    let status = response.status();
    let status_colored = if status.is_success() {
        status.as_str().green()
    } else if status.is_client_error() {
        status.as_str().yellow()
    } else if status.is_server_error() {
        status.as_str().red()
    } else {
        status.as_str().cyan()
    };
    println!("Status: {} ({})", status_colored, status.canonical_reason().unwrap_or("").dimmed());
    println!("Time: {:?}", duration);

    // Print Response Headers
    println!("{}", "Headers:".magenta());
    for (name, value) in response.headers() {
        println!("  {}: {}", name.as_str().cyan(), value.to_str()?.dimmed());
    }

    // Print Response Body
    println!("{}", "Body:".magenta());
    let response_body = response.text().await?;
    if response_body.is_empty() {
        println!("{}", "(Empty response body)".dimmed());
    } else {
        // Attempt to pretty-print if JSON, otherwise print plain text
        match serde_json::from_str::<serde_json::Value>(&response_body) {
            Ok(json_value) => {
                match serde_json::to_string_pretty(&json_value) {
                    Ok(pretty_json) => println!("{}", pretty_json.green()),
                    Err(_) => println!("{}", response_body), // Fallback to plain text if pretty print fails
                }
            }
            Err(_) => {
                // Not JSON, print as plain text
                println!("{}", response_body);
            }
        }
    }

    Ok(())
} 