use colored::Colorize;
use std::io::{self, Write};

// Helper function to prompt user for input
pub fn prompt(message: &str) -> io::Result<String> {
    print!("{}: ", message.cyan());
    io::stdout().flush()?; // Ensure the prompt message is displayed before input
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

// Add other utility functions here later (e.g., parsing human sizes) 