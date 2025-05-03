use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug, Clone)]
pub struct UnitConverterArgs {
    #[command(subcommand)]
    pub command: UnitConverterCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum UnitConverterCommands {
    /// Convert length units
    Length {
        /// Value to convert
        value: f64,
        /// Unit to convert from (e.g., km, m, mi, ft)
        from_unit: String,
        /// Unit to convert to (e.g., km, m, mi, ft)
        to_unit: String,
    },
    // Add other categories like Mass, Temperature, Currency later
}

pub fn handle_unit_converter_command(args: UnitConverterArgs) -> Result<String> {
    match args.command {
        UnitConverterCommands::Length { value, from_unit, to_unit } => {
            convert_length(value, &from_unit, &to_unit)
        }
    }
}

fn convert_length(value: f64, from_unit: &str, to_unit: &str) -> Result<String> {
    const KM_TO_MILES: f64 = 0.621371;
    const METERS_TO_FEET: f64 = 3.28084;

    let result = match (from_unit.to_lowercase().as_str(), to_unit.to_lowercase().as_str()) {
        ("km", "mi") | ("kilometers", "miles") => value * KM_TO_MILES,
        ("mi", "km") | ("miles", "kilometers") => value / KM_TO_MILES,
        ("m", "ft") | ("meters", "feet") => value * METERS_TO_FEET,
        ("ft", "m") | ("feet", "meters") => value / METERS_TO_FEET,
        ("km", "m") | ("kilometers", "meters") => value * 1000.0,
        ("m", "km") | ("meters", "kilometers") => value / 1000.0,
        ("mi", "ft") | ("miles", "feet") => value * 5280.0,
        ("ft", "mi") | ("feet", "miles") => value / 5280.0,
         // Add more conversions as needed: m <-> mi, km <-> ft etc. via intermediate conversions
        (f, t) if f == t => value, // Same unit
        _ => return Err(anyhow::anyhow!("Unsupported length conversion: {} to {}", from_unit, to_unit)),
    };

    Ok(format!("{} {} = {:.4} {}", value, from_unit, result, to_unit))
}

// Add functions for other conversions (mass, temp, currency) here

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_conversion() {
        assert!(convert_length(1.0, "km", "mi").unwrap().contains("0.6214"));
        assert!(convert_length(1.0, "mi", "km").unwrap().contains("1.6093"));
        assert!(convert_length(1.0, "m", "ft").unwrap().contains("3.2808"));
        assert!(convert_length(1.0, "ft", "m").unwrap().contains("0.3048"));
        assert!(convert_length(10.0, "km", "km").unwrap().contains("10.0000"));
        assert!(convert_length(1.0, "km", "m").unwrap().contains("1000.0000"));
        assert!(convert_length(5280.0, "ft", "mi").unwrap().contains("1.0000"));
    }

    #[test]
    fn test_invalid_length_conversion() {
        assert!(convert_length(1.0, "km", "kg").is_err());
    }
} 