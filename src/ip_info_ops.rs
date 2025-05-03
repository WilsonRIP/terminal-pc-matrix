use anyhow::Result;
use serde_json::Value;
use colored::*;

/// Retrieves information about an IP address, including geolocation, ASN, and abuse contacts
pub async fn lookup_ip_info(ip: &str, show_abuse: bool, show_asn: bool) -> Result<()> {
    println!("Looking up information for IP: {}", ip.cyan());
    
    // Use ipinfo.io API for the lookup
    let url = format!("https://ipinfo.io/{}/json", ip);
    let client = reqwest::Client::new();
    let response = client.get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("API request failed with status: {}", response.status()));
    }
    
    let result: Value = response.json().await?;
    display_ip_info(&result, show_abuse, show_asn)?;
    
    Ok(())
}

fn display_ip_info(data: &Value, show_abuse: bool, show_asn: bool) -> Result<()> {
    println!("\n{}", "IP Information".magenta().bold());
    println!("---------------");
    
    // Extract and display the basic information
    if let Some(ip) = data.get("ip").and_then(|v| v.as_str()) {
        println!("{}: {}", "IP".green(), ip);
    }
    
    if let Some(hostname) = data.get("hostname").and_then(|v| v.as_str()) {
        println!("{}: {}", "Hostname".green(), hostname);
    }
    
    // Location information
    if let Some(city) = data.get("city").and_then(|v| v.as_str()) {
        println!("{}: {}", "City".green(), city);
    }
    
    if let Some(region) = data.get("region").and_then(|v| v.as_str()) {
        println!("{}: {}", "Region".green(), region);
    }
    
    if let Some(country) = data.get("country").and_then(|v| v.as_str()) {
        println!("{}: {}", "Country".green(), country);
    }
    
    if let Some(loc) = data.get("loc").and_then(|v| v.as_str()) {
        println!("{}: {}", "Location".green(), loc);
    }
    
    if let Some(postal) = data.get("postal").and_then(|v| v.as_str()) {
        println!("{}: {}", "Postal".green(), postal);
    }
    
    if let Some(timezone) = data.get("timezone").and_then(|v| v.as_str()) {
        println!("{}: {}", "Timezone".green(), timezone);
    }
    
    // Network information
    if let Some(org) = data.get("org").and_then(|v| v.as_str()) {
        println!("{}: {}", "Organization".green(), org);
    }
    
    // ASN information (if requested)
    if show_asn {
        println!("\n{}", "ASN Information".magenta().bold());
        println!("---------------");
        
        if let Some(asn) = data.get("asn") {
            if let Some(asn_id) = asn.get("asn").and_then(|v| v.as_str()) {
                println!("{}: {}", "ASN".green(), asn_id);
            }
            
            if let Some(name) = asn.get("name").and_then(|v| v.as_str()) {
                println!("{}: {}", "ASN Name".green(), name);
            }
            
            if let Some(domain) = asn.get("domain").and_then(|v| v.as_str()) {
                println!("{}: {}", "ASN Domain".green(), domain);
            }
            
            if let Some(route) = asn.get("route").and_then(|v| v.as_str()) {
                println!("{}: {}", "ASN Route".green(), route);
            }
            
            if let Some(asn_type) = asn.get("type").and_then(|v| v.as_str()) {
                println!("{}: {}", "ASN Type".green(), asn_type);
            }
        } else {
            println!("{}", "No ASN information available".yellow());
        }
    }
    
    // Abuse contact information (if requested)
    if show_abuse {
        println!("\n{}", "Abuse Contact Information".magenta().bold());
        println!("-------------------------");
        
        if let Some(abuse) = data.get("abuse") {
            if let Some(address) = abuse.get("address").and_then(|v| v.as_str()) {
                println!("{}: {}", "Abuse Email".green(), address);
            }
            
            if let Some(phone) = abuse.get("phone").and_then(|v| v.as_str()) {
                println!("{}: {}", "Abuse Phone".green(), phone);
            }
            
            if let Some(network) = abuse.get("network").and_then(|v| v.as_str()) {
                println!("{}: {}", "Network".green(), network);
            }
        } else {
            println!("{}", "No abuse contact information available".yellow());
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require network access and may be brittle.
    // They are marked `ignore` by default.

    #[tokio::test]
    #[ignore]
    async fn test_lookup_google_dns() {
        let result = lookup_ip_info("8.8.8.8", false, false).await;
        assert!(result.is_ok());
    }
} 