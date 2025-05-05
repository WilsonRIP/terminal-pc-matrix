// Export all modules so they can be used by the Tauri application
pub mod antivirus_ops;
pub mod audio_text_ops;
pub mod browser_ops;
pub mod calculator_ops;
pub mod cli;
pub mod dns_ops;
pub mod file_download_ops;
pub mod file_ops;
pub mod http_ops;
pub mod image_download_ops;
pub mod interactive;
pub mod ip_info_ops;
pub mod network_ops;
pub mod pc_specs_ops;
pub mod system_ops;
pub mod unit_converter_ops;
pub mod utils;
pub mod video_download_ops;
pub mod whois_ops;

// Re-export common functions/types for easier access
pub use file_ops::*;
pub use network_ops::*;
pub use pc_specs_ops::*;
pub use system_ops::*; 