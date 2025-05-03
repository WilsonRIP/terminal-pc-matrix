use colored::*;
use dirs;
use fs_extra::dir as fsx_dir;
use humansize::{format_size, DECIMAL};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use regex::Regex;
use ring::digest::{Context, Digest, SHA256};
use data_encoding::HEXUPPER;
use crate::cli::{RenameArgs, SyncArgs};

// --- Existing File Ops ---

// Function to list directory contents
pub fn list_directory(path: &Path) -> io::Result<()> {
    if !path.is_dir() {
        eprintln!(
            "{}",
            format!("Error: '{}' is not a valid directory.", path.display()).red().bold()
        );
        return Err(io::Error::new(io::ErrorKind::NotADirectory, "Path is not a directory"));
    }
    let absolute_path = path.canonicalize()?;
    println!(
        "{}",
        format!("Contents of: {}", absolute_path.display()).magenta().bold()
    );
    println!("{:<35} {:<15} {:>15}", "Name".cyan().bold(), "Type".cyan().bold(), "Size".cyan().bold());
    println!("{}", "-".repeat(67).dimmed());

    let mut entries = Vec::new();
    for entry_result in fs::read_dir(path)? {
        match entry_result {
            Ok(entry) => entries.push(entry),
            Err(e) => eprintln!("{}", format!("Error reading entry: {}", e).red()),
        }
    }
    entries.sort_by_key(|dir_entry| dir_entry.file_name());

    for entry in entries {
        let entry_path = entry.path();
        let file_name_os = entry.file_name();
        let file_name = file_name_os.to_string_lossy();
        match fs::metadata(&entry_path) {
            Ok(metadata) => {
                let file_type_str = if metadata.is_dir() { "Dir".blue().bold() } else if metadata.is_file() { "File".normal() } else { "Link/Other".dimmed() };
                let size_str = if metadata.is_file() {
                    format_size(metadata.len(), DECIMAL)
                } else {
                    "-".dimmed().to_string()
                };
                let name_display = if metadata.is_dir() { file_name.blue().bold() } else { file_name.normal() };
                println!("{:<35} {:<15} {:>15}", name_display, file_type_str, size_str);
            }
            Err(e) => {
                eprintln!("{}", format!("Error accessing metadata for '{}': {}", file_name, e).red());
                println!("{:<35} {:<15} {:>15}", file_name.red(), "Error".red().bold(), "-".red());
            }
        }
    }
    Ok(())
}

// Function to backup a directory
pub fn backup_directory(source: &Path, destination: &Path) -> Result<(), fs_extra::error::Error> {
    if !source.is_dir() {
        eprintln!("{}", format!("Error: Source '{}' is not a valid directory.", source.display()).red().bold());
    }
    if let Some(parent) = destination.parent() {
        if !parent.exists() {
            println!("Destination parent directory '{}' does not exist. Creating...", parent.display().to_string().yellow());
            match fs::create_dir_all(parent) {
                Ok(_) => println!("Created destination parent: {}", parent.display().to_string().green()),
                Err(e) => {
                    eprintln!("{}", format!("Error creating destination parent '{}': {}", parent.display(), e).red().bold());
                    return Err(fs_extra::error::Error::new(fs_extra::error::ErrorKind::Other, &format!("Failed to create destination parent: {}", e)));
                }
            }
        }
    }
    println!("{}", format!("Starting backup: '{}' -> '{}'...", source.display().to_string().cyan(), destination.display().to_string().cyan()));
    let mut options = fsx_dir::CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    match fsx_dir::copy(source, destination, &options) {
        Ok(bytes_copied) => {
            println!("{}", format!("Success: Copied {} to '{}'", format_size(bytes_copied, DECIMAL), destination.display()).green().bold());
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", format!("Backup Error: {}", e).red().bold());
            Err(e)
        }
    }
}

// Function to organize screenshots on macOS Desktop
#[cfg(target_os = "macos")]
pub fn organize_screenshots() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "Organizing macOS Desktop screenshots...".cyan());
    let desktop_dir = dirs::desktop_dir().ok_or("Desktop directory not found")?;
    let screenshots_dir = desktop_dir.join("Screenshots");

    if !screenshots_dir.exists() {
        println!("Creating directory: {}", screenshots_dir.display().to_string().yellow());
        fs::create_dir_all(&screenshots_dir).map_err(|e| format!("Error creating Screenshots directory '{}': {}", screenshots_dir.display(), e))?;
        println!("Directory created: {}", screenshots_dir.display().to_string().green());
    } else {
        println!("Found directory: {}", screenshots_dir.display().to_string().dimmed());
    }

    let mut moved_count = 0;
    let mut error_count = 0;
    println!("Scanning Desktop: {}", desktop_dir.display().to_string().cyan());

    for entry_result in fs::read_dir(&desktop_dir)? {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if (filename.starts_with("Screen Shot ") || filename.starts_with("Screenshot ")) && filename.ends_with(".png") {
                            let destination = screenshots_dir.join(filename);
                            println!("  Moving '{}' -> {}", filename.dimmed(), screenshots_dir.file_name().unwrap_or_default().to_string_lossy().blue());
                            match fs::rename(&path, &destination) {
                                Ok(_) => moved_count += 1,
                                Err(e) => {
                                    eprintln!("{}", format!("    Error moving '{}': {}", filename, e).red());
                                    error_count += 1;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", format!("Error reading Desktop entry: {}", e).red());
                error_count += 1;
            }
        }
    }

    if moved_count > 0 { println!("{}", format!("Successfully moved {} screenshot(s).", moved_count).green()); }
    if error_count > 0 { println!("{}", format!("Encountered {} error(s).", error_count).yellow()); }
    if moved_count == 0 && error_count == 0 { println!("{}", "No new screenshots found on the Desktop to move.".dimmed()); }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn organize_screenshots() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    eprintln!("{}", "Organize screenshots is only supported on macOS.".yellow().bold());
    Err("Feature not supported on this OS".into())
}

// Helper to filter out directory walk errors we can ignore
fn is_permission_error(entry: &Result<DirEntry, walkdir::Error>) -> bool {
    if let Err(e) = entry {
        if e.io_error().map_or(false, |io_err| io_err.kind() == io::ErrorKind::PermissionDenied) {
            if let Some(path) = e.path() {
                eprintln!("{}: {}", "Permission denied (skipping)".yellow(), path.display().to_string().dimmed());
            }
            return true;
        }
        eprintln!("{}: {:?}", "Walkdir error (will attempt to continue)".red(), e);
    }
    false
}

// Function for disk analysis
pub fn analyze_disk(path_to_analyze: &Path, top: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", format!("Analyzing disk usage for '{}', showing top {}...", path_to_analyze.display(), top).cyan());
    let mut files: Vec<(u64, PathBuf)> = Vec::new();
    let mut error_count = 0;

    let walker = WalkDir::new(path_to_analyze)
        .into_iter()
        .filter_entry(|e| !is_permission_error(&Ok(e.clone())))
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();
        if path.is_file() {
            match fs::metadata(path) {
                Ok(metadata) => files.push((metadata.len(), path.to_path_buf())),
                Err(e) => {
                    eprintln!("{}: {} - {}", "Error reading metadata".red(), path.display(), e);
                    error_count += 1;
                }
            }
        }
    }

    files.sort_by(|a, b| b.0.cmp(&a.0));

    println!("\n{}:", format!("Top {} Largest Files Found", std::cmp::min(top, files.len())).magenta().bold());
    if files.is_empty() && error_count == 0 {
        println!("{}", "No files found in the specified path.".dimmed());
    } else {
        for (size, path) in files.iter().take(top) {
            println!("  {} - {}", format_size(*size, DECIMAL).green(), path.display());
        }
    }

    if error_count > 0 { println!("\n{}", format!("Encountered {} error(s) reading file metadata.", error_count).yellow()); }

    println!("\n{}: Directory size analysis is not yet implemented.", "Note".yellow());
    Ok(())
}

// Helper function to calculate directory size
pub fn calculate_dir_size(path: &Path) -> (u64, u32, u32) {
    let walker = WalkDir::new(path).into_iter();
    let mut total_size: u64 = 0;
    let mut file_count: u32 = 0;
    let mut error_count: u32 = 0;

    for entry_result in walker.filter_entry(|e| !is_permission_error(&Ok(e.clone()))).filter_map(|e| e.ok()) {
        if entry_result.file_type().is_file() {
            match entry_result.metadata() {
                Ok(metadata) => { total_size += metadata.len(); file_count += 1; },
                Err(e) => {
                    eprintln!("Error getting metadata for size calc: {:?} - {:?}", entry_result.path(), e);
                    error_count += 1;
                }
            }
        }
    }
    (total_size, file_count, error_count)
}

// Function for system cleaning (identify only for now)
pub fn clean_system(dry_run: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = if dry_run { "(Dry Run)".yellow() } else { "".normal() };
    println!("{} Identifying temporary/cache files {}...", "EXPERIMENTAL:".yellow().bold(), mode);

    if !dry_run {
        println!("{}", "WARNING: Actual file deletion is NOT IMPLEMENTED. Forcing Dry Run.".red().bold());
    }

    let mut locations_to_check: Vec<(&str, Option<PathBuf>)> = Vec::new();

    locations_to_check.push(("User Cache Dir", dirs::cache_dir()));
    locations_to_check.push(("System Temp Dir", Some(std::env::temp_dir())));

    #[cfg(target_os = "macos")] {
        if let Some(home) = dirs::home_dir() { locations_to_check.push(("macOS ~/Library/Caches", Some(home.join("Library/Caches")))); }
        locations_to_check.push(("macOS /private/var/tmp", Some(PathBuf::from("/private/var/tmp"))));
    }
    #[cfg(target_os = "windows")] {
        if let Some(app_data) = std::env::var("LOCALAPPDATA").ok() {
            locations_to_check.push(("Windows Local AppData Temp", Some(PathBuf::from(app_data).join("Temp"))));
        }
    }
    #[cfg(target_os = "linux")] {
        locations_to_check.push(("Linux /tmp", Some(PathBuf::from("/tmp"))));
        locations_to_check.push(("Linux /var/tmp", Some(PathBuf::from("/var/tmp"))));
        if let Some(home) = dirs::home_dir() { locations_to_check.push(("User ~/.cache", Some(home.join(".cache")))); }
    }

    println!("\n{}:", "Potential Temporary Locations".magenta().bold());
    let mut total_potential_size: u64 = 0;
    let mut total_errors: u32 = 0;

    for (description, path_option) in locations_to_check {
        match path_option {
            Some(path) => {
                if path.exists() && path.is_dir() {
                    println!("\nChecking: {} ({})", description.dimmed(), path.display().to_string().cyan());
                    let (size, file_count, errors) = calculate_dir_size(&path);
                    total_errors += errors;

                    if size > 0 || file_count > 0 {
                        println!("  Size: {}, Files: {}", format_size(size, DECIMAL).green(), file_count.to_string().green());
                        if errors > 0 {
                            println!("  {}", format!("(Encountered {} errors reading dir contents)", errors).yellow());
                        }
                        total_potential_size += size;
                    } else if errors > 0 {
                        println!("  {} {}", "Empty or inaccessible.".dimmed(), format!("({} errors reading)", errors).yellow());
                    } else {
                        println!("  {}", "Empty.".dimmed());
                    }
                } else {
                    println!("\nSkipping: {} (Path not found or not a directory: {})", description.yellow(), path.display());
                }
            }
            None => println!("\nSkipping: {} (Could not determine path)", description.yellow()),
        }
    }
    println!("\n{}", "-".repeat(40).dimmed());
    println!("Total potential size identified: {}", format_size(total_potential_size, DECIMAL).bold().green());
    if total_errors > 0 { println!("Encountered {} errors during size calculation.", total_errors.to_string().yellow()); }
    if dry_run { println!("\n{}. No files were deleted.", "Dry run complete".bold().green()); }
    Ok(())
}

// Batch Rename Files
pub fn rename_files(args: &RenameArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = if args.dry_run { "(Dry Run)".yellow() } else { "".normal() };
    println!(
        "{} Batch Renaming in '{}' {}...",
        "Running:".cyan(),
        args.directory.display(),
        mode
    );
    println!("Pattern: '{}'", args.pattern.dimmed());
    println!("Replacement: '{}'", args.replacement.dimmed());

    let re = Regex::new(&args.pattern).map_err(|e| format!("Invalid Regex Pattern: {}", e))?;
    let mut rename_count = 0;
    let mut error_count = 0;
    let mut skipped_count = 0;

    for entry_result in fs::read_dir(&args.directory)? {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if re.is_match(filename) {
                            let new_filename = re.replace_all(filename, &args.replacement[..]).to_string();
                            if new_filename != filename {
                                let new_path = args.directory.join(&new_filename);
                                println!("  Rename '{}' -> '{}'", filename.dimmed(), new_filename.green());
                                if !args.dry_run {
                                    if new_path.exists() {
                                        eprintln!("    {}: '{}' already exists. Skipping.", "Warning".yellow(), new_filename);
                                        skipped_count += 1;
                                        continue;
                                    }
                                    match fs::rename(&path, &new_path) {
                                        Ok(_) => rename_count += 1,
                                        Err(e) => {
                                            eprintln!("    {}: {}", "Error renaming".red(), e);
                                            error_count += 1;
                                        }
                                    }
                                } else {
                                    if new_path.exists() {
                                        println!("    {}: '{}' already exists (potential conflict).", "Warning".yellow(), new_filename);
                                        skipped_count += 1;
                                    } else {
                                        rename_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}: {}", "Error reading directory entry".red(), e);
                error_count += 1;
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if args.dry_run {
        println!("{} file(s) would be renamed.", rename_count.to_string().green());
    } else {
        println!("{} file(s) successfully renamed.", rename_count.to_string().green());
    }
    if skipped_count > 0 {
        println!("{} file(s) skipped (target existed).", skipped_count.to_string().yellow());
    }
    if error_count > 0 {
        println!("{} error(s) occurred.", error_count.to_string().yellow());
    }
    if rename_count == 0 && skipped_count == 0 && error_count == 0 {
        println!("{}", "No files matched the pattern or required renaming.".dimmed());
    }

    Ok(())
}

// Helper to parse human-readable size string (e.g., "1k", "10M", "2G")
fn parse_size(size_str: &str) -> Result<u64, String> {
    let size_str = size_str.trim().to_lowercase();
    let num_part = size_str.trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.');
    let unit_part = size_str.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');

    let num: f64 = num_part.parse().map_err(|_| format!("Invalid number format in size: '{}'", num_part))?;

    let multiplier = match unit_part {
        "" | "b" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return Err(format!("Invalid size unit (use b, k, m, g, t): '{}'", unit_part)),
    };

    if num < 0.0 {
        return Err("Size cannot be negative".to_string());
    }

    Ok((num * multiplier).round() as u64)
}

// Helper to calculate SHA256 hash of a file
fn hash_file(path: &Path) -> io::Result<Digest> {
    let file = fs::File::open(path)?;
    let mut reader = io::BufReader::new(file);
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 8192];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(context.finish())
}

// Find Duplicate Files
pub fn find_duplicates(path_to_search: &Path, min_size_str: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let min_size = parse_size(min_size_str).map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(format!("Invalid minimum size: {}", e)))?;
    println!(
        "{} Scanning '{}' for duplicate files larger than {}...",
        "🔍".cyan(),
        path_to_search.display(),
        format_size(min_size, DECIMAL).yellow()
    );

    let mut files_by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut error_count = 0;
    let mut potential_dup_files = 0;
    let mut hashed_files = 0;

    println!("{}", "Phase 1: Grouping files by size...".dimmed());
    let walker = WalkDir::new(path_to_search)
        .into_iter()
        .filter_entry(|e| !is_permission_error(&Ok(e.clone())))
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();
        if path.is_file() {
            match fs::metadata(path) {
                Ok(metadata) => {
                    let size = metadata.len();
                    if size >= min_size {
                        files_by_size.entry(size).or_default().push(path.to_path_buf());
                    }
                }
                Err(e) => {
                    eprintln!("{}: {} - {}", "Error reading metadata".red(), path.display(), e);
                    error_count += 1;
                }
            }
        }
    }

    for paths in files_by_size.values() {
        if paths.len() > 1 {
            potential_dup_files += paths.len();
        }
    }
    println!("Found {} potential duplicate file(s) based on size.", potential_dup_files.to_string().yellow());

    println!("{}", "Phase 2: Hashing potential duplicates...".dimmed());
    for (_, paths) in files_by_size.into_iter() {
        if paths.len() > 1 {
            for path in paths {
                hashed_files += 1;
                match hash_file(&path) {
                    Ok(digest) => {
                        let hash_string = HEXUPPER.encode(digest.as_ref());
                        hash_map.entry(hash_string).or_default().push(path);
                    }
                    Err(e) => {
                        eprintln!("{}: {} - {}", "Error hashing file".red(), path.display(), e);
                        error_count += 1;
                    }
                }
            }
        }
    }
    println!("Hashed {} file(s).", hashed_files.to_string().dimmed());

    let duplicate_sets: Vec<Vec<PathBuf>> = hash_map
        .into_values()
        .filter(|paths| paths.len() > 1)
        .collect();

    println!("{}", "-".repeat(40).dimmed());
    if duplicate_sets.is_empty() {
        println!("{}", "No duplicate files found.".green());
    } else {
        println!("Found {} set(s) of duplicate files:", duplicate_sets.len().to_string().yellow());
        for (i, set) in duplicate_sets.iter().enumerate() {
            println!("\n{}. Set ({} files):", format!("{}", i + 1).magenta(), set.len());
            for path in set {
                println!("  - {}", path.display());
            }
        }
    }
    if error_count > 0 {
        println!("\nEncountered {} error(s) during process.", error_count.to_string().yellow());
    }

    Ok(())
}

// Sync Folders (One-Way)
pub fn sync_folders(args: &SyncArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mode = if args.dry_run { "(Dry Run)".yellow() } else { "".normal() };
    let delete_mode = if args.delete { " (with delete)".yellow() } else { "".normal() };

    println!(
        "{} Syncing '{}' -> '{}'{}{}...",
        "Running:".cyan(),
        args.source.display(),
        args.destination.display(),
        delete_mode,
        mode
    );

    if !args.source.is_dir() {
        return Err(anyhow::anyhow!("Source '{}' is not a valid directory.", args.source.display()).into());
    }

    if !args.dry_run && !args.destination.exists() {
        println!("Creating destination directory: {}", args.destination.display().to_string().yellow());
        if let Err(e) = fs::create_dir_all(&args.destination) {
            return Err(anyhow::anyhow!("Failed to create destination directory '{}': {}", args.destination.display(), e).into());
        }
        println!("Destination created: {}", args.destination.display().to_string().green());
    } else if !args.destination.is_dir() && args.destination.exists() {
        return Err(anyhow::anyhow!("Destination '{}' exists but is not a directory.", args.destination.display()).into());
    }

    let mut copied_count = 0;
    let mut updated_count = 0;
    let mut deleted_count = 0;
    let mut error_count = 0;
    let mut src_relative_paths: HashMap<PathBuf, fs::Metadata> = HashMap::new();

    println!("{}", "Phase 1: Scanning source & updating destination...".dimmed());
    for entry_result in WalkDir::new(&args.source).into_iter().filter_map(|e| e.ok()) {
        let src_path = entry_result.path();
        let relative_path = match src_path.strip_prefix(&args.source) {
             Ok(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
             _ => continue,
        };

        let dest_path = args.destination.join(&relative_path);

        match fs::metadata(src_path) {
            Ok(src_meta) => {
                src_relative_paths.insert(relative_path.clone(), src_meta.clone());

                if src_meta.is_dir() {
                    if !args.dry_run && !dest_path.exists() {
                        println!("  Creating directory: {}", dest_path.display().to_string().cyan());
                        if let Err(e) = fs::create_dir_all(&dest_path) {
                            eprintln!("    {}: {}", "Error creating directory".red(), e);
                            error_count += 1;
                        }
                    }
                } else if src_meta.is_file() {
                    match fs::metadata(&dest_path) {
                        Ok(dest_meta) => {
                            if !dest_meta.is_file() {
                                eprintln!("    {}: Destination '{}' exists but is not a file. Skipping update.", "Error".red(), dest_path.display());
                                error_count += 1;
                            } else if src_meta.len() != dest_meta.len() || src_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH) > dest_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH) {
                                println!("  Updating file: {}", dest_path.display().to_string().yellow());
                                if !args.dry_run {
                                    match fs::copy(src_path, &dest_path) {
                                        Ok(_) => updated_count += 1,
                                        Err(e) => {
                                            eprintln!("    {}: {}", "Error updating file".red(), e);
                                            error_count += 1;
                                        }
                                    }
                                } else {
                                    updated_count += 1;
                                }
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                            println!("  Copying new file: {}", dest_path.display().to_string().green());
                            if !args.dry_run {
                                if let Some(parent) = dest_path.parent() {
                                    if !parent.exists() {
                                        if let Err(e) = fs::create_dir_all(parent) {
                                            eprintln!("    {}: Failed to create parent dir '{}': {}", "Error".red(), parent.display(), e);
                                            error_count += 1;
                                            continue;
                                        }
                                    }
                                }
                                match fs::copy(src_path, &dest_path) {
                                    Ok(_) => copied_count += 1,
                                    Err(e) => {
                                        eprintln!("    {}: {}", "Error copying file".red(), e);
                                        error_count += 1;
                                    }
                                }
                            } else {
                                copied_count += 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("{}: Failed to read metadata for '{}': {}", "Error".red(), dest_path.display(), e);
                            error_count += 1;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}: {} - {}", "Error reading source metadata".red(), src_path.display(), e);
                error_count += 1;
            }
        }
    }

    if args.delete {
         println!("{}", "\nPhase 2: Scanning destination for extra items...".dimmed());
         for entry_result in WalkDir::new(&args.destination).contents_first(true).into_iter().filter_map(|e| e.ok()) {
             let dest_path = entry_result.path();
             let relative_path = match dest_path.strip_prefix(&args.destination) {
                 Ok(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
                 _ => continue,
             };

             if !src_relative_paths.contains_key(&relative_path) {
                 println!("  Deleting extra item: {}", dest_path.display().to_string().red());
                 if !args.dry_run {
                     match fs::metadata(dest_path) {
                         Ok(meta) => {
                             if meta.is_dir() {
                                 if let Err(e) = fs::remove_dir(dest_path) {
                                     if e.kind() != io::ErrorKind::NotFound {
                                        eprintln!("    {}: Could not delete directory '{}' (maybe not empty?): {}", "Error".red(), dest_path.display(), e);
                                        error_count += 1;
                                     }
                                 } else {
                                     deleted_count += 1;
                                 }
                             } else {
                                 if let Err(e) = fs::remove_file(dest_path) {
                                     if e.kind() != io::ErrorKind::NotFound {
                                        eprintln!("    {}: Could not delete file '{}': {}", "Error".red(), dest_path.display(), e);
                                        error_count += 1;
                                     }
                                 } else {
                                     deleted_count += 1;
                                 }
                             }
                         }
                          Err(ref e) if e.kind() == io::ErrorKind::NotFound => { /* Already deleted, ignore */ }
                          Err(e) => {
                             eprintln!("    {}: Failed to read metadata for deletion '{}': {}", "Error".red(), dest_path.display(), e);
                             error_count += 1;
                         }
                     }
                } else {
                     deleted_count += 1;
                }
            }
         }
    }

    println!("{}", "-".repeat(40).dimmed());
    println!(
        "Sync {}. Copied: {}, Updated: {}, Deleted: {}",
        if args.dry_run { "Dry Run Complete".yellow() } else { "Complete".green() },
        copied_count.to_string().green(),
        updated_count.to_string().yellow(),
        deleted_count.to_string().red()
    );
     if error_count > 0 {
        println!("{} error(s) occurred during sync.", error_count.to_string().yellow());
    }

    Ok(())
}

// Search Files by Name
pub fn search_files(path_to_search: &Path, query: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "{} Searching for '{}' in '{}'...",
        "Running:".cyan(),
        query.yellow(),
        path_to_search.display()
    );

    let mut found_files: Vec<PathBuf> = Vec::new();
    let query_lower = query.to_lowercase();

    let walker = WalkDir::new(path_to_search)
        .into_iter()
        .filter_entry(|e| !is_permission_error(&Ok(e.clone())))
        .filter_map(|e| e.ok());

    for entry in walker {
        if let Some(filename) = entry.file_name().to_str() {
            if filename.to_lowercase().contains(&query_lower) {
                found_files.push(entry.path().to_path_buf());
            }
        }
    }

    println!("{}", "-".repeat(40).dimmed());
    if found_files.is_empty() {
        println!("{}", "No files found matching the query.".dimmed());
    } else {
        println!("Found {} file(s) matching '{}':", found_files.len().to_string().green(), query.yellow());
        found_files.sort();
        for path in found_files {
            println!("  - {}", path.display());
        }
    }

    Ok(())
} 