use anyhow::Result;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

// Performs a WHOIS lookup for the given domain.
pub async fn lookup_domain(domain: &str) -> Result<String> {
    println!("Looking up WHOIS for: {}", domain);

    // Extract TLD for server selection
    let tld = extract_tld(domain);
    let server = get_whois_server(&tld);
    
    // Connect to the WHOIS server directly
    match query_whois_server(server, domain) {
        Ok(result) => Ok(result),
        Err(e) => Err(anyhow::anyhow!("WHOIS lookup failed: {}", e)),
    }
}

// Extract the TLD from a domain name
fn extract_tld(domain: &str) -> String {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 1].to_string()
    } else {
        "com".to_string() // Default fallback
    }
}

// Get the appropriate WHOIS server for a TLD
fn get_whois_server(tld: &str) -> &str {
    match tld {
        "com" => "whois.verisign-grs.com",
        "net" => "whois.verisign-grs.com",
        "org" => "whois.pir.org",
        "io" => "whois.nic.io",
        "dev" => "whois.nic.dev",
        "ai" => "whois.nic.ai",
        "co" => "whois.nic.co",
        "uk" => "whois.nic.uk",
        "ru" => "whois.tcinet.ru",
        "jp" => "whois.jprs.jp",
        "cn" => "whois.cnnic.cn",
        "fr" => "whois.nic.fr",
        "nl" => "whois.domain-registry.nl",
        "de" => "whois.denic.de",
        "au" => "whois.auda.org.au",
        _ => "whois.iana.org",  // Default WHOIS server
    }
}

// Query a WHOIS server directly via TCP
fn query_whois_server(server: &str, domain: &str) -> Result<String> {
    // Connect to server on port 43 (standard WHOIS port)
    let address = format!("{}:43", server);
    let mut stream = TcpStream::connect(&address)?;
    
    // Set reasonable timeout
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    
    // Send the query (domain name followed by \r\n)
    let query = format!("{}\r\n", domain);
    stream.write_all(query.as_bytes())?;
    
    // Read the response
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require network access and may be brittle
    // depending on domain availability and WHOIS server responses.
    // They are marked `ignore` by default.

    #[tokio::test]
    #[ignore]
    async fn test_whois_lookup_google() {
        let result = lookup_domain("google.com").await;
        assert!(result.is_ok());
        let output = result.unwrap().to_lowercase();
        // Basic check for common WHOIS fields
        assert!(output.contains("domain name: google.com"));
        assert!(output.contains("registrar:"));
        assert!(output.contains("creation date:"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_whois_lookup_nonexistent() {
        // Expecting an error or a specific "not found" message
        // The exact error might vary depending on the TLD and registrar.
        let result = lookup_domain("thisdomainprobablyshouldnotexist12345.com").await;
        // We might get an Err, or an Ok with a "No match" message.
        if let Ok(output) = result {
            assert!(output.to_lowercase().contains("no match"));
        } 
        // Or assert!(result.is_err()); // Depending on expected behavior
    }
} 