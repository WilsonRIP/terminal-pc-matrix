use gtk4::{prelude::*, Application, ApplicationWindow, Label, Box, Orientation, Stack, StackSidebar, TextView, ScrolledWindow, Button, Align, Notebook, FileChooserWidget, FileChooserAction, ListBox, SelectionMode};
use crate::pc_specs_ops; // Import the pc_specs_ops module
use crate::file_ops; // Import the file_ops module
use std::path::PathBuf;
use glib::clone; // Import glib::clone for closures
use std::cell::RefCell;

const APP_ID: &str = "com.wilsoniirip.terminalpcmatrix";

// Helper function to create the PC Specs page
fn create_pc_specs_page() -> Box {
    let container = Box::new(Orientation::Vertical, 10);
    container.set_margin_top(10);
    container.set_margin_bottom(10);
    container.set_margin_start(10);
    container.set_margin_end(10);

    let button = Button::with_label("Get PC Specs");
    button.set_halign(Align::Center);

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let text_view = TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .build();

    scrolled_window.set_child(Some(&text_view));

    // --- Connect Button Click --- 
    let text_view_clone = text_view.clone();
    button.connect_clicked(move |_| {
        match pc_specs_ops::get_system_info_string() {
            Ok(specs_string) => {
                let buffer = text_view_clone.buffer();
                buffer.set_text(&specs_string);
            }
            Err(e) => {
                let buffer = text_view_clone.buffer();
                buffer.set_text(&format!("Error getting PC specs:\n{}", e));
                eprintln!("Error getting PC specs: {}", e);
            }
        }
    });
    // --- End Connect Button Click ---

    container.append(&button);
    container.append(&scrolled_window);

    container
}

// Helper function to create the 'List Directory' tab content
fn create_list_dir_tab() -> Box {
    let container = Box::new(Orientation::Vertical, 10);
    container.set_margin_top(10);
    container.set_margin_bottom(10);
    container.set_margin_start(10);
    container.set_margin_end(10);

    let controls_box = Box::new(Orientation::Horizontal, 10);
    let file_chooser = FileChooserWidget::new(FileChooserAction::SelectFolder);
    let list_button = Button::with_label("List Contents");
    controls_box.append(&file_chooser);
    controls_box.append(&list_button);

    let results_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic) 
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .build();

    let list_box = ListBox::new();
    list_box.set_selection_mode(SelectionMode::None); // Disable selection
    results_scrolled_window.set_child(Some(&list_box));

    container.append(&controls_box);
    container.append(&results_scrolled_window);

    // --- Connect List Button Click ---
    let list_box_clone = list_box.clone();
    let file_chooser_clone = file_chooser.clone(); // Clone file_chooser too

    list_button.connect_clicked(move |_| {
        // Clear previous results
        while let Some(child) = list_box_clone.first_child() {
            list_box_clone.remove(&child);
        }

        if let Some(file) = file_chooser_clone.file() {
            if let Some(path) = file.path() {
                match file_ops::get_directory_listing(&path) {
                    Ok(file_infos) => {
                        if file_infos.is_empty() {
                            let label = Label::new(Some("Directory is empty."));
                            list_box_clone.append(&label);
                        } else {
                            for info in file_infos {
                                // Create a simple label for each entry
                                let label_text = format!("{} [{}] ({})", info.name, info.file_type, info.size_human);
                                let label = Label::new(Some(&label_text));
                                label.set_halign(Align::Start);
                                list_box_clone.append(&label);
                            }
                        }
                    }
                    Err(e) => {
                        let error_label = Label::new(Some(&format!("Error listing directory:\n{}", e)));
                        // Optionally add CSS class for error styling
                        // error_label.add_css_class("error-text"); 
                        list_box_clone.append(&error_label);
                        eprintln!("Error listing directory: {}", e);
                    }
                }
            } else {
                 let error_label = Label::new(Some("Error: Could not get path from file chooser."));
                 list_box_clone.append(&error_label);
            }
        } else {
             let error_label = Label::new(Some("Error: No directory selected."));
             list_box_clone.append(&error_label);
        }
    });
    // --- End Connect List Button Click ---

    container
}

// Helper function to create the 'Backup Directory' tab content
fn create_backup_dir_tab() -> Box {
    let container = Box::new(Orientation::Vertical, 15); // Added spacing
    container.set_margin_top(15);
    container.set_margin_bottom(15);
    container.set_margin_start(15);
    container.set_margin_end(15);

    let source_chooser = FileChooserWidget::new(FileChooserAction::SelectFolder);
    source_chooser.set_halign(Align::Fill); // Make it fill width

    let dest_chooser = FileChooserWidget::new(FileChooserAction::SelectFolder);
    dest_chooser.set_halign(Align::Fill); // Make it fill width

    let backup_button = Button::with_label("Start Backup");
    backup_button.set_halign(Align::Center);
    backup_button.set_margin_top(10); // Add some space above the button

    let status_label = Label::new(Some("Select source and destination directories."));
    status_label.set_halign(Align::Center);
    status_label.set_margin_top(10);
    status_label.set_wrap(true); // Allow wrapping for longer messages

    // Backup button click handler
    backup_button.connect_clicked(clone!(@weak status_label, @weak source_chooser, @weak dest_chooser => move |_| {
        status_label.set_text("Starting backup..."); // Immediate feedback

        let source_file = source_chooser.file();
        let dest_file = dest_chooser.file();

        match (source_file, dest_file) {
            (Some(source), Some(dest)) => {
                if let (Some(source_path), Some(dest_path)) = (source.path(), dest.path()) {
                    // NOTE: This runs synchronously and will block the UI for large backups.
                    // Consider glib::spawn_blocking for long operations.
                    match file_ops::backup_directory(&source_path, &dest_path) {
                        Ok(_) => {
                             status_label.set_markup(&format!(
                                "<span color='green'><b>Success:</b> Backup completed to '{}'</span>",
                                dest_path.display()
                            ));
                        }
                        Err(e) => {
                             status_label.set_markup(&format!(
                                "<span color='red'><b>Error:</b> {}</span>",
                                glib::markup_escape_text(&e.to_string()) // Escape error message
                            ));
                            eprintln!("Backup Error: {}", e); // Also log to console
                        }
                    }
                } else {
                    status_label.set_markup("<span color='orange'><b>Warning:</b> Could not get path from source or destination file chooser.</span>");
                }
            }
            (None, _) => {
                status_label.set_markup("<span color='orange'><b>Warning:</b> Please select a source directory.</span>");
            }
            (_, None) => {
                status_label.set_markup("<span color='orange'><b>Warning:</b> Please select a destination directory.</span>");
            }
        }
    }));


    container.append(&Label::new(Some("Source Directory:")));
    container.append(&source_chooser);
    container.append(&Label::new(Some("Destination Directory:"))); // Added label for clarity
    container.append(&dest_chooser);
    container.append(&backup_button);
    container.append(&status_label);

    container
}

// Helper function to create the File Operations page (with Notebook)
fn create_file_ops_page() -> Notebook {
    let notebook = Notebook::new();

    // Tab 1: List Directory
    let list_dir_page = create_list_dir_tab();
    let list_dir_label = Label::new(Some("List Directory"));
    notebook.append_page(&list_dir_page, Some(&list_dir_label));

    // Tab 2: Backup Directory
    let backup_dir_page = create_backup_dir_tab();
    let backup_dir_label = Label::new(Some("Backup Directory"));
    notebook.append_page(&backup_dir_page, Some(&backup_dir_label));

    // Add more tabs here later (Organize, Analyze, Clean, etc.)

    notebook
}

pub fn build_ui(app: &Application) {
    // --- Main Application Window --- 
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Terminal PC Matrix")
        .default_width(800)
        .default_height(600)
        .build();

    // --- Main Container Box (Sidebar + Stack) ---
    let main_box = Box::new(Orientation::Horizontal, 0);

    // --- Stack for Pages ---
    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    // --- Sidebar for Stack Navigation ---
    let sidebar = StackSidebar::new();
    sidebar.set_stack(&stack);

    // --- Create and Add Pages to Stack ---

    // 1. PC Specs Page
    let pc_specs_page = create_pc_specs_page();
    stack.add_titled(&pc_specs_page, Some("pc_specs"), "PC Specs");

    // 2. File Operations Page (with Notebook)
    let file_ops_page = create_file_ops_page();
    stack.add_titled(&file_ops_page, Some("file_ops"), "File Operations");

    // --- TODO: Add other main pages here (Network, Downloads, etc.) ---
    let network_label = Label::new(Some("Network Operations UI Goes Here"));
    stack.add_titled(&network_label, Some("network_ops"), "Network Operations");

    // --- Assemble Main Layout ---
    main_box.append(&sidebar);
    main_box.append(&stack);

    // --- Set main_box as the window's child ---
    window.set_child(Some(&main_box));

    // Present the window
    window.present();
}

pub fn run_app() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
} 