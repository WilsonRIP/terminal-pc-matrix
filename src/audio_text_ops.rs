use anyhow::{anyhow, Result};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
// use simple_transcribe_rs::{model_handler::ModelHandler, transcriber::Transcriber};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::Builder;

// Available whisper models from smallest to largest
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelSize {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
}

impl ModelSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "tiny",
            ModelSize::Base => "base",
            ModelSize::Small => "small",
            ModelSize::Medium => "medium",
            ModelSize::Large => "large",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tiny" => Some(ModelSize::Tiny),
            "base" => Some(ModelSize::Base),
            "small" => Some(ModelSize::Small),
            "medium" => Some(ModelSize::Medium),
            "large" => Some(ModelSize::Large),
            _ => None,
        }
    }
}

pub struct TranscriptionOptions {
    pub model_size: ModelSize,
    pub output_file: Option<PathBuf>,
    pub save_timestamps: bool,
    pub output_srt: bool,
    pub output_txt: bool,
}

impl Default for TranscriptionOptions {
    fn default() -> Self {
        Self {
            model_size: ModelSize::Base,
            output_file: None,
            save_timestamps: true,
            output_srt: true,
            output_txt: true,
        }
    }
}

/// Transcribes audio from a file to text using whisper-rs
pub async fn transcribe_audio(
    _audio_file: &Path,
    _options: TranscriptionOptions,
) -> Result<String> {
    // Show progress because model loading might take time
    // let pb = ProgressBar::new_spinner();
    // pb.set_style(
    //     ProgressStyle::default_spinner()
    //         .template("{spinner:.green} {msg}")
    //         .unwrap(),
    // );
    // pb.set_message("Loading transcription model...");
    // pb.enable_steady_tick(Duration::from_millis(100));

    // // Create models directory if it doesn't exist
    // let models_dir = Path::new("models");
    // if !models_dir.exists() {
    //     fs::create_dir_all(models_dir)?;
    // }

    // // Initialize the model handler (will download the model if not present)
    // let model_handler = ModelHandler::new(options.model_size.as_str(), "models/").await;
    
    // pb.set_message("Transcribing audio...");

    // // Create the transcriber
    // let transcriber = Transcriber::new(model_handler);
    
    // // Transcribe the audio
    // let audio_path_str = audio_file.to_string_lossy().to_string();
    // let result = transcriber.transcribe(&audio_path_str, None)
    //     .map_err(|e| anyhow!("Transcription failed: {}", e))?;
    
    // let transcript = result.get_text();
    // pb.finish_with_message(format!("{}", "Transcription completed!".green()));

    // if options.output_txt || options.output_srt {
    //     save_transcription_outputs(&result, audio_file, &options)?;
    // }

    // Ok(transcript.to_string())
    Err(anyhow!("Audio transcription temporarily disabled."))
}

/// Save transcription outputs (txt and/or srt files)
fn save_transcription_outputs(
    _result: &impl std::fmt::Debug,
    _audio_file: &Path,
    _options: &TranscriptionOptions,
) -> Result<()> {
    // let base_path = if let Some(output_path) = &options.output_file {
    //     output_path.clone()
    // } else {
    //     let stem = audio_file.file_stem().unwrap_or_default().to_string_lossy();
    //     let current_dir = std::env::current_dir()?;
    //     current_dir.join(stem.to_string())
    // };

    // // Save plain text transcript (Temporarily disable using result)
    // if options.output_txt {
    //     let txt_path = base_path.with_extension("txt");
    //     fs::write(&txt_path, "Transcript text unavailable due to type inference issue.")?; 
    //     println!("{} {}", "Saved transcript to:".green(), txt_path.display());
    // }

    // // Save SRT subtitle file (Temporarily disable using result)
    // if options.output_srt {
    //     let srt_path = base_path.with_extension("srt");
    //     let srt_content = format!("1\n00:00:00,000 --> 00:00:01,000\nSRT generation unavailable due to type inference issue.\n");
    //     fs::write(&srt_path, srt_content)?;
    //     println!("{} {}", "Saved subtitles to:".green(), srt_path.display());
    // }

    Ok(())
}

/// Generate SRT subtitle file content from transcription result
fn generate_srt(_result: &impl std::fmt::Debug) -> String {
    // format!("1\n00:00:00,000 --> 00:00:01,000\nSRT generation unavailable due to type inference issue.\n")
    String::new()
}

/// Save audio from video before transcription
pub async fn extract_audio_from_video(_video_path: &Path) -> Result<PathBuf> {
    // // This function would extract audio from video using a library like ffmpeg
    // // For this implementation, we'll just simulate the process
    // println!("{} {}", "Extracting audio from video:".cyan(), video_path.display());
    
    // // Create a temporary file for the extracted audio
    // let temp_dir = Builder::new().prefix("audio_extract").tempdir()?;
    // let audio_path = temp_dir.path().join("extracted_audio.mp3");
    
    // // Here we would actually extract the audio using ffmpeg or similar tool
    // // For now, just pretend we did and return the path
    // // In a real implementation, you would call a command like:
    // // ffmpeg -i video.mp4 -q:a 0 -map a extracted_audio.mp3
    
    // Ok(audio_path)
     Err(anyhow!("Audio extraction temporarily disabled."))
}

/// Handle audio transcription process
pub async fn handle_audio_transcription(
    _input_path: &Path,
    _options: TranscriptionOptions,
) -> Result<String> {
    // let audio_path = if mime_guess::from_path(input_path).first_raw().map_or(false, |mime| mime.starts_with("video/")) {
    //     // Extract audio from video first
    //     extract_audio_from_video(input_path).await?
    // } else {
    //     input_path.to_path_buf()
    // };

    // // Perform the actual transcription
    // transcribe_audio(&audio_path, options).await
    Err(anyhow!("Audio transcription temporarily disabled."))
} 