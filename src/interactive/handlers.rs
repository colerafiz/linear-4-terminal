use super::app::InteractiveApp;
use super::event::{Event, EventHandler};
use crate::config::get_api_key;
use crossterm::{
    event::KeyCode,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io;
use std::process::Command;

pub async fn run_interactive_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Check API key first
    get_api_key()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = InteractiveApp::new().await?;
    let events = EventHandler::new(100);

    // Main loop
    let mut launch_editor_next_frame = false;
    let mut delete_issue_next_frame: Option<String> = None;

    loop {
        // Handle deletion before drawing
        if let Some(issue_id) = delete_issue_next_frame.take() {
            let _ = app.delete_issue(&issue_id).await;
        }

        // Handle external editor mode before drawing
        if launch_editor_next_frame {
            launch_editor_next_frame = false;
            let current_content = app.edit_input.clone();

            // Debug: Log the content length
            eprintln!(
                "DEBUG: Launching editor with content length: {}",
                current_content.len()
            );

            let edited_content = launch_external_editor(&mut terminal, &current_content)?;
            app.handle_external_editor_result(edited_content);
            // Force a redraw after returning from editor
            terminal.draw(|f| super::ui::draw(f, &app))?;
        }

        // Draw UI
        terminal.draw(|f| super::ui::draw(f, &app))?;

        // Handle events
        match events.recv()? {
            Event::Key(key_event) => {
                match key_event.code {
                    KeyCode::Char('r') if app.mode == super::app::AppMode::Normal => {
                        // Refresh issues
                        let _ = app.refresh_issues().await;
                    }
                    KeyCode::Char('d') if app.mode == super::app::AppMode::Normal => {
                        // Toggle done status
                        let _ = app.toggle_done().await;
                    }
                    KeyCode::Enter if app.mode == super::app::AppMode::Comment => {
                        // Submit comment
                        let _ = app.submit_comment().await;
                    }
                    KeyCode::Enter if app.mode == super::app::AppMode::EditField => {
                        // Submit edit
                        let _ = app.submit_edit().await;
                    }
                    KeyCode::Enter if app.mode == super::app::AppMode::SelectOption => {
                        // Submit selection
                        let _ = app.submit_edit().await;
                    }
                    KeyCode::Char('e') | KeyCode::Char('E')
                        if app.mode == super::app::AppMode::Edit && app.edit_field_index == 1 =>
                    {
                        // Set the edit field to Description before launching editor
                        app.edit_field = super::app::EditField::Description;
                        // Launch external editor for description
                        if app.prepare_external_editor().is_some() {
                            launch_editor_next_frame = true;
                        }
                    }
                    KeyCode::Enter if app.mode == super::app::AppMode::Confirm => {
                        // Handle Enter key confirmation based on selection
                        if app.confirm_selection {
                            // User selected "Yes"
                            if let Some(action) = app.confirm_action.take() {
                                match action {
                                    super::app::ConfirmAction::DeleteIssue(issue_id) => {
                                        delete_issue_next_frame = Some(issue_id);
                                        app.mode = super::app::AppMode::Normal;
                                        app.confirm_message.clear();
                                        app.confirm_selection = false;
                                    }
                                }
                            }
                        } else {
                            // User selected "No"
                            app.mode = super::app::AppMode::Normal;
                            app.confirm_action = None;
                            app.confirm_message.clear();
                            app.confirm_selection = false;
                        }
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y')
                        if app.mode == super::app::AppMode::Confirm =>
                    {
                        // Handle 'y' key for quick confirmation
                        if let Some(action) = app.confirm_action.take() {
                            match action {
                                super::app::ConfirmAction::DeleteIssue(issue_id) => {
                                    delete_issue_next_frame = Some(issue_id);
                                    app.mode = super::app::AppMode::Normal;
                                    app.confirm_message.clear();
                                    app.confirm_selection = false;
                                }
                            }
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc
                        if app.mode == super::app::AppMode::Confirm =>
                    {
                        app.mode = super::app::AppMode::Normal;
                        app.confirm_action = None;
                        app.confirm_message.clear();
                        app.confirm_selection = false;
                    }
                    _ => app.handle_key(key_event.code),
                }
            }
            Event::Tick => {
                // Handle any periodic updates here
            }
        }

        // Check if we should launch editor
        if app.mode == super::app::AppMode::ExternalEditor && !launch_editor_next_frame {
            launch_editor_next_frame = true;
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn launch_external_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    content: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // Create a temporary file with .md extension for better editor support
    let temp_file = tempfile::Builder::new().suffix(".md").tempfile()?;

    // Write content to the file
    std::fs::write(temp_file.path(), content)?;

    // Debug: Verify content was written
    eprintln!(
        "DEBUG: Wrote {} bytes to {}",
        content.len(),
        temp_file.path().display()
    );

    // Get the editor from environment or use defaults
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| {
            // Try to find a suitable editor, preferring helix
            if Command::new("which")
                .arg("hx")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                "hx".to_string()
            } else if Command::new("which")
                .arg("helix")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                "helix".to_string()
            } else if Command::new("which")
                .arg("nano")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                "nano".to_string()
            } else if Command::new("which")
                .arg("vim")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                "vim".to_string()
            } else if Command::new("which")
                .arg("vi")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                "vi".to_string()
            } else {
                "nano".to_string() // fallback
            }
        });

    // Suspend the TUI
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    terminal.show_cursor()?;

    // Clear the terminal to ensure clean state
    println!("\n");

    // Debug: Log which editor we're using
    eprintln!("DEBUG: Launching editor: {}", editor);

    // Launch the editor
    let status = Command::new(&editor).arg(temp_file.path()).status();

    // Restore the TUI
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;

    // Force a full redraw
    terminal.clear()?;

    // Check if editor ran successfully
    match status {
        Ok(status) if status.success() => {
            // Read the edited content
            let edited_content = std::fs::read_to_string(temp_file.path())?;
            // Trim trailing whitespace that editors might add
            Ok(Some(edited_content.trim_end().to_string()))
        }
        Ok(_) => {
            // User likely cancelled (e.g., :q! in vim)
            Ok(None)
        }
        Err(e) => {
            // Failed to launch editor
            eprintln!("Failed to launch editor '{}': {}", editor, e);
            Ok(None)
        }
    }
}
