use crate::client::LinearClient;
use crate::config::get_api_key;
use crate::models::{Issue, WorkflowState};
use crossterm::event::KeyCode;
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    Filter,
    Detail,
    Comment,
    Edit,
    EditField,
    SelectOption,
    ExternalEditor,
    Links,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupBy {
    Status,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditField {
    Title,
    Description,
    Status,
    Assignee,
    Priority,
    Labels,
}

pub struct InteractiveApp {
    pub mode: AppMode,
    pub issues: Vec<Issue>,
    pub filtered_issues: Vec<Issue>,
    pub selected_index: usize,
    pub group_by: GroupBy,
    pub search_query: String,
    #[allow(dead_code)]
    pub filter_query: String,
    pub should_quit: bool,
    pub client: LinearClient,
    pub loading: bool,
    pub error_message: Option<String>,
    pub comment_input: String,
    pub comment_cursor_position: usize,
    pub selected_issue_id: Option<String>,
    pub edit_field: EditField,
    pub edit_input: String,
    pub edit_field_index: usize,
    pub workflow_states: Vec<WorkflowState>,
    pub available_labels: Vec<crate::models::issue::Label>,
    pub selected_labels: Vec<String>, // IDs of selected labels
    pub option_index: usize,
    pub selected_option: Option<String>,
    pub cursor_position: usize,
    pub external_editor_field: Option<EditField>,
    pub current_issue_links: Vec<String>,
    pub selected_link_index: usize,
    pub confirm_message: String,
    pub confirm_action: Option<ConfirmAction>,
    pub confirm_selection: bool, // false = No (default), true = Yes
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteIssue(String),
}

impl InteractiveApp {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let api_key = get_api_key()?;
        let client = LinearClient::new(api_key);

        let mut app = Self {
            mode: AppMode::Normal,
            issues: Vec::new(),
            filtered_issues: Vec::new(),
            selected_index: 0,
            group_by: GroupBy::Status,
            search_query: String::new(),
            filter_query: String::new(),
            should_quit: false,
            client,
            loading: true,
            error_message: None,
            comment_input: String::new(),
            comment_cursor_position: 0,
            selected_issue_id: None,
            edit_field: EditField::Title,
            edit_input: String::new(),
            edit_field_index: 0,
            workflow_states: Vec::new(),
            available_labels: Vec::new(),
            selected_labels: Vec::new(),
            option_index: 0,
            selected_option: None,
            cursor_position: 0,
            external_editor_field: None,
            current_issue_links: Vec::new(),
            selected_link_index: 0,
            confirm_message: String::new(),
            confirm_action: None,
            confirm_selection: false,
        };

        // Make all three API calls in parallel for faster startup
        let (issues_result, states_result, labels_result) = tokio::join!(
            app.client.get_issues(None, Some(100)),
            app.client.get_workflow_states(),
            app.client.get_labels()
        );

        // Handle issues result
        match issues_result {
            Ok(issues) => {
                app.issues = issues;
                app.apply_filters();
            }
            Err(e) => {
                app.error_message = Some(format!("Failed to load issues: {}", e));
                return Err(e);
            }
        }

        // Handle workflow states result
        match states_result {
            Ok(states) => {
                app.workflow_states = states;
            }
            Err(e) => {
                eprintln!("Failed to fetch workflow states: {}", e);
                app.workflow_states = Vec::new();
            }
        }

        // Handle labels result
        match labels_result {
            Ok(labels) => {
                app.available_labels = labels;
            }
            Err(e) => {
                eprintln!("Failed to fetch labels: {}", e);
                app.available_labels = Vec::new();
            }
        }

        app.loading = false;
        Ok(app)
    }

    pub async fn refresh_issues(&mut self) -> Result<(), Box<dyn Error>> {
        self.loading = true;
        self.error_message = None;

        match self.client.get_issues(None, Some(100)).await {
            Ok(issues) => {
                self.issues = issues;
                self.apply_filters();
                self.loading = false;
                Ok(())
            }
            Err(e) => {
                self.loading = false;
                self.error_message = Some(format!("Failed to load issues: {}", e));
                Err(e)
            }
        }
    }

    pub fn apply_filters(&mut self) {
        self.filtered_issues = self.issues.clone();

        // Apply search filter
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            self.filtered_issues.retain(|issue| {
                issue.title.to_lowercase().contains(&query)
                    || issue.identifier.to_lowercase().contains(&query)
            });
        }

        // Apply sorting based on group_by
        match self.group_by {
            GroupBy::Status => {
                self.filtered_issues.sort_by(|a, b| {
                    a.state
                        .name
                        .cmp(&b.state.name)
                        .then(a.priority.cmp(&b.priority).reverse())
                });
            }
            GroupBy::Project => {
                self.filtered_issues.sort_by(|a, b| {
                    let a_project = a.project.as_ref().map(|p| &p.name);
                    let b_project = b.project.as_ref().map(|p| &p.name);
                    a_project
                        .cmp(&b_project)
                        .then(a.state.name.cmp(&b.state.name))
                        .then(a.priority.cmp(&b.priority).reverse())
                });
            }
        }

        // Reset selection if needed
        if self.selected_index >= self.filtered_issues.len() && !self.filtered_issues.is_empty() {
            self.selected_index = self.filtered_issues.len() - 1;
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode_key(key),
            AppMode::Search => self.handle_search_mode_key(key),
            AppMode::Filter => self.handle_filter_mode_key(key),
            AppMode::Detail => self.handle_detail_mode_key(key),
            AppMode::Comment => self.handle_comment_mode_key(key),
            AppMode::Edit => self.handle_edit_mode_key(key),
            AppMode::EditField => self.handle_edit_field_mode_key(key),
            AppMode::SelectOption => self.handle_select_option_mode_key(key),
            AppMode::ExternalEditor => {} // External editor is handled in the main loop
            AppMode::Links => self.handle_links_mode_key(key),
            AppMode::Confirm => self.handle_confirm_mode_key(key),
        }
    }

    fn handle_normal_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_selection_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection_up(),
            KeyCode::Char('g') => self.toggle_group_by(),
            KeyCode::Char('/') => self.mode = AppMode::Search,
            KeyCode::Char('f') => self.mode = AppMode::Filter,
            KeyCode::Char('o') => {
                // Open current issue in Linear
                if let Some(issue) = self.get_selected_issue() {
                    let _ = self.open_link(&issue.url);
                }
            }
            KeyCode::Char('e') => {
                // Edit current issue
                if let Some(issue) = self.get_selected_issue() {
                    self.selected_issue_id = Some(issue.id.clone());
                    self.edit_field_index = 0;
                    self.mode = AppMode::Edit;
                }
            }
            KeyCode::Enter => {
                if !self.filtered_issues.is_empty() {
                    self.mode = AppMode::Detail;
                    // Update current issue links
                    if let Some(issue) = self.get_selected_issue() {
                        self.current_issue_links = super::ui::get_issue_links(issue);
                        self.selected_link_index = 0;
                    }
                }
            }
            KeyCode::Char('r') => {
                // Refresh issues - handled in main loop
            }
            KeyCode::Char('s') => {
                // Quick edit status
                if let Some(issue) = self.get_selected_issue() {
                    self.selected_issue_id = Some(issue.id.clone());
                    self.edit_field = EditField::Status;
                    self.option_index = 0;
                    self.selected_option = None;
                    self.mode = AppMode::SelectOption;
                }
            }
            KeyCode::Char('c') => {
                // Quick comment
                if let Some(issue) = self.get_selected_issue() {
                    self.selected_issue_id = Some(issue.id.clone());
                    self.comment_input.clear();
                    self.comment_cursor_position = 0;
                    self.mode = AppMode::Comment;
                }
            }
            KeyCode::Char('l') => {
                // Quick edit labels - go directly to label selection
                if let Some(issue) = self.get_selected_issue() {
                    let issue_id = issue.id.clone();
                    let current_label_ids: Vec<String> = issue
                        .labels
                        .nodes
                        .iter()
                        .map(|label| label.id.clone())
                        .collect();

                    self.selected_issue_id = Some(issue_id);
                    self.edit_field = EditField::Labels;
                    self.option_index = 0;
                    self.selected_option = None;
                    self.selected_labels = current_label_ids;
                    self.mode = AppMode::SelectOption;
                }
            }
            KeyCode::Char('d') => {
                // Toggle done status (between completed and unstarted/started)
                if let Some(issue) = self.get_selected_issue() {
                    self.selected_issue_id = Some(issue.id.clone());
                    // We'll trigger the toggle in the async handler
                }
            }
            KeyCode::Char('D') => {
                // Delete (archive) current issue with confirmation
                if let Some(issue) = self.get_selected_issue() {
                    let issue_title = issue.title.clone();
                    let issue_id = issue.id.clone();
                    self.confirm_message = format!(
                        "Are you sure you want to delete issue \"{}\"? (y/n)",
                        issue_title
                    );
                    self.confirm_action = Some(ConfirmAction::DeleteIssue(issue_id));
                    self.confirm_selection = false; // Default to "No" for safety
                    self.mode = AppMode::Confirm;
                }
            }
            _ => {}
        }
    }

    fn handle_search_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.search_query.clear();
                self.apply_filters();
            }
            KeyCode::Enter => {
                self.mode = AppMode::Normal;
                self.apply_filters();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.apply_filters();
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.apply_filters();
            }
            _ => {}
        }
    }

    fn handle_filter_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_detail_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::Normal;
            }
            KeyCode::Char('c') => {
                if let Some(issue) = self.get_selected_issue() {
                    self.selected_issue_id = Some(issue.id.clone());
                    self.comment_input.clear();
                    self.comment_cursor_position = 0;
                    self.mode = AppMode::Comment;
                }
            }
            KeyCode::Char('e') => {
                if let Some(issue) = self.get_selected_issue() {
                    self.selected_issue_id = Some(issue.id.clone());
                    self.edit_field_index = 0;
                    self.mode = AppMode::Edit;
                }
            }
            KeyCode::Char('o') => {
                // Open Linear issue URL
                if !self.current_issue_links.is_empty() {
                    let _ = self.open_link(&self.current_issue_links[0]);
                }
            }
            KeyCode::Char('l') => {
                // Enter links navigation mode
                if self.current_issue_links.len() > 1 {
                    self.selected_link_index = 0;
                    self.mode = AppMode::Links;
                }
            }
            KeyCode::Char(c) if c.is_digit(10) => {
                // Open numbered link
                let index = c.to_digit(10).unwrap() as usize;
                if index < self.current_issue_links.len() {
                    let _ = self.open_link(&self.current_issue_links[index]);
                }
            }
            _ => {}
        }
    }

    fn handle_comment_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Detail;
                self.comment_input.clear();
                self.comment_cursor_position = 0;
            }
            KeyCode::Enter => {
                // Comment submission will be handled in the main loop
                // because it's async
            }
            KeyCode::Char(c) => {
                self.comment_input.insert(self.comment_cursor_position, c);
                self.comment_cursor_position += 1;
            }
            KeyCode::Backspace => {
                if self.comment_cursor_position > 0 {
                    self.comment_input.remove(self.comment_cursor_position - 1);
                    self.comment_cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.comment_cursor_position < self.comment_input.len() {
                    self.comment_input.remove(self.comment_cursor_position);
                }
            }
            KeyCode::Left => {
                if self.comment_cursor_position > 0 {
                    self.comment_cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.comment_cursor_position < self.comment_input.len() {
                    self.comment_cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.comment_cursor_position = 0;
            }
            KeyCode::End => {
                self.comment_cursor_position = self.comment_input.len();
            }
            _ => {}
        }
    }

    fn move_selection_down(&mut self) {
        if !self.filtered_issues.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_issues.len();
        }
    }

    fn move_selection_up(&mut self) {
        if !self.filtered_issues.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_issues.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    fn toggle_group_by(&mut self) {
        self.group_by = match self.group_by {
            GroupBy::Status => GroupBy::Project,
            GroupBy::Project => GroupBy::Status,
        };
        // Re-apply filters to trigger re-sorting
        self.apply_filters();
    }

    pub fn get_selected_issue(&self) -> Option<&Issue> {
        self.filtered_issues.get(self.selected_index)
    }

    fn open_link(&self, url: &str) -> Result<(), Box<dyn Error>> {
        #[cfg(target_os = "macos")]
        let cmd = "open";
        #[cfg(target_os = "windows")]
        let cmd = "start";
        #[cfg(target_os = "linux")]
        let cmd = "xdg-open";

        std::process::Command::new(cmd)
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open link: {}", e))?;

        Ok(())
    }

    pub async fn submit_comment(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(issue_id) = &self.selected_issue_id {
            if !self.comment_input.trim().is_empty() {
                self.loading = true;
                match self
                    .client
                    .create_comment(issue_id, &self.comment_input)
                    .await
                {
                    Ok(_) => {
                        self.loading = false;
                        self.comment_input.clear();
                        self.mode = AppMode::Detail;
                        Ok(())
                    }
                    Err(e) => {
                        self.loading = false;
                        self.error_message = Some(format!("Failed to add comment: {}", e));
                        Err(e)
                    }
                }
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn handle_edit_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::Detail;
                self.edit_input.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.edit_field_index > 0 {
                    self.edit_field_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.edit_field_index < 5 {
                    // We have 6 fields (0-5)
                    self.edit_field_index += 1;
                }
            }
            KeyCode::Enter => {
                self.edit_field = match self.edit_field_index {
                    0 => EditField::Title,
                    1 => EditField::Description,
                    2 => EditField::Status,
                    3 => EditField::Assignee,
                    4 => EditField::Priority,
                    5 => EditField::Labels,
                    _ => EditField::Title,
                };
                self.edit_input.clear();

                // For status, priority, and labels, show selection mode
                match self.edit_field {
                    EditField::Status | EditField::Priority | EditField::Labels => {
                        self.option_index = 0;
                        self.selected_option = None;

                        // For labels, populate selected_labels with current issue's labels
                        if self.edit_field == EditField::Labels {
                            if let Some(issue) = self.get_selected_issue() {
                                self.selected_labels = issue
                                    .labels
                                    .nodes
                                    .iter()
                                    .map(|label| label.id.clone())
                                    .collect();
                            } else {
                                self.selected_labels.clear();
                            }
                        }

                        self.mode = AppMode::SelectOption;
                    }
                    _ => {
                        // Pre-fill with current value for text fields
                        if let Some(issue) = self.get_selected_issue() {
                            self.edit_input = match self.edit_field {
                                EditField::Title => issue.title.clone(),
                                EditField::Description => {
                                    // For description, provide a template if empty
                                    let desc = issue.description.clone().unwrap_or_default();
                                    if desc.trim().is_empty() {
                                        "".to_string() // Start with empty for new descriptions
                                    } else {
                                        desc
                                    }
                                }
                                EditField::Assignee => issue
                                    .assignee
                                    .as_ref()
                                    .map(|a| a.name.clone())
                                    .unwrap_or_default(),
                                _ => String::new(),
                            };
                        }
                        self.cursor_position = self.edit_input.len();
                        self.mode = AppMode::EditField;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_edit_field_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Edit;
                self.edit_input.clear();
                self.cursor_position = 0;
            }
            KeyCode::Enter => {
                // Submit edit - will be handled in main loop
            }
            KeyCode::Char('\x05') => {
                // Ctrl+E - launch external editor for description
                if self.edit_field == EditField::Description {
                    self.prepare_external_editor();
                }
            }
            KeyCode::Char(c) => {
                self.edit_input.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.edit_input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                if self.cursor_position < self.edit_input.len() {
                    self.edit_input.remove(self.cursor_position);
                }
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_position < self.edit_input.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.edit_input.len();
            }
            _ => {}
        }
    }

    fn handle_select_option_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::Edit;
                self.option_index = 0;
                self.selected_option = None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.option_index > 0 {
                    self.option_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_index = match self.edit_field {
                    EditField::Status => self.workflow_states.len().saturating_sub(1),
                    EditField::Priority => 4, // 0-4 for None, Low, Medium, High, Urgent
                    EditField::Labels => self.available_labels.len().saturating_sub(1),
                    _ => 0,
                };
                if self.option_index < max_index {
                    self.option_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                match self.edit_field {
                    EditField::Status => {
                        if let Some(state) = self.workflow_states.get(self.option_index) {
                            self.selected_option = Some(state.id.clone());
                        }
                    }
                    EditField::Priority => {
                        self.selected_option = Some(self.option_index.to_string());
                    }
                    EditField::Labels => {
                        // Toggle label selection with space bar
                        if let Some(label) = self.available_labels.get(self.option_index) {
                            let label_id = label.id.clone();
                            if let Some(pos) =
                                self.selected_labels.iter().position(|id| id == &label_id)
                            {
                                self.selected_labels.remove(pos);
                            } else {
                                self.selected_labels.push(label_id);
                            }
                            // Don't close menu on space, only on Enter
                            if key == KeyCode::Char(' ') {
                                return;
                            }
                        }
                    }
                    _ => {}
                }
                // Submit will be handled in main loop
            }
            _ => {}
        }
    }

    fn handle_links_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::Detail;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_link_index > 0 {
                    self.selected_link_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_link_index < self.current_issue_links.len().saturating_sub(1) {
                    self.selected_link_index += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char('o') => {
                if let Some(link) = self.current_issue_links.get(self.selected_link_index) {
                    let _ = self.open_link(link);
                }
            }
            _ => {}
        }
    }

    pub fn prepare_external_editor(&mut self) -> Option<String> {
        if self.edit_field == EditField::Description {
            // If edit_input is empty, populate it with current description
            if self.edit_input.is_empty() {
                if let Some(issue) = self.get_selected_issue() {
                    self.edit_input = issue.description.clone().unwrap_or_default();
                }
            }

            self.external_editor_field = Some(self.edit_field);
            self.mode = AppMode::ExternalEditor;
            Some(self.edit_input.clone())
        } else {
            None
        }
    }

    pub fn handle_external_editor_result(&mut self, content: Option<String>) {
        if let Some(new_content) = content {
            self.edit_input = new_content;
        }
        self.mode = AppMode::EditField;
        self.external_editor_field = None;
    }

    pub async fn submit_edit(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(issue_id) = &self.selected_issue_id {
            self.loading = true;

            let result = match self.edit_field {
                EditField::Title => {
                    if !self.edit_input.trim().is_empty() {
                        self.client
                            .update_issue(
                                issue_id,
                                Some(&self.edit_input),
                                None,
                                None,
                                None,
                                None,
                                None,
                            )
                            .await
                    } else {
                        self.loading = false;
                        return Ok(());
                    }
                }
                EditField::Description => {
                    self.client
                        .update_issue(
                            issue_id,
                            None,
                            Some(&self.edit_input),
                            None,
                            None,
                            None,
                            None,
                        )
                        .await
                }
                EditField::Status => {
                    if let Some(state_id) = &self.selected_option {
                        self.client
                            .update_issue(issue_id, None, None, Some(state_id), None, None, None)
                            .await
                    } else {
                        self.loading = false;
                        return Ok(());
                    }
                }
                EditField::Priority => {
                    if let Some(priority_str) = &self.selected_option {
                        if let Ok(priority) = priority_str.parse::<u8>() {
                            self.client
                                .update_issue(
                                    issue_id,
                                    None,
                                    None,
                                    None,
                                    Some(priority),
                                    None,
                                    None,
                                )
                                .await
                        } else {
                            self.loading = false;
                            return Ok(());
                        }
                    } else {
                        self.loading = false;
                        return Ok(());
                    }
                }
                EditField::Assignee => {
                    // For now, assignee still uses text input
                    self.loading = false;
                    self.error_message = Some("Assignee field is not yet editable".to_string());
                    return Ok(());
                }
                EditField::Labels => {
                    let label_ids: Vec<&str> =
                        self.selected_labels.iter().map(|s| s.as_str()).collect();
                    self.client
                        .update_issue(issue_id, None, None, None, None, None, Some(label_ids))
                        .await
                }
            };

            match result {
                Ok(_) => {
                    self.loading = false;
                    self.edit_input.clear();
                    self.selected_option = None;
                    self.selected_labels.clear();
                    self.mode = AppMode::Detail;
                    // Refresh issues to show the update
                    let _ = self.refresh_issues().await;
                }
                Err(e) => {
                    self.loading = false;
                    self.error_message = Some(format!("Failed to update: {}", e));
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_done(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(issue_id) = &self.selected_issue_id {
            if let Some(issue) = self.get_selected_issue() {
                let current_state_type = issue.state.state_type.clone();

                self.loading = true;
                self.error_message = None;

                // Find the appropriate state to toggle to
                let new_state_id = if current_state_type == "completed" {
                    // If completed, find an unstarted or started state
                    self.workflow_states
                        .iter()
                        .find(|s| s.state_type == "unstarted" || s.state_type == "started")
                        .or_else(|| {
                            self.workflow_states
                                .iter()
                                .find(|s| s.state_type == "backlog")
                        })
                        .map(|s| s.id.clone())
                } else {
                    // If not completed, find a completed state
                    self.workflow_states
                        .iter()
                        .find(|s| s.state_type == "completed")
                        .map(|s| s.id.clone())
                };

                if let Some(state_id) = new_state_id {
                    match self
                        .client
                        .update_issue(issue_id, None, None, Some(&state_id), None, None, None)
                        .await
                    {
                        Ok(_) => {
                            self.loading = false;
                            // Refresh issues to show the update
                            let _ = self.refresh_issues().await;
                            Ok(())
                        }
                        Err(e) => {
                            self.loading = false;
                            self.error_message =
                                Some(format!("Failed to toggle done status: {}", e));
                            Err(e)
                        }
                    }
                } else {
                    self.loading = false;
                    self.error_message = Some("No appropriate workflow state found".to_string());
                    Err("No appropriate workflow state found".into())
                }
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub async fn delete_issue(&mut self, issue_id: &str) -> Result<(), Box<dyn Error>> {
        self.loading = true;
        self.error_message = None;

        match self.client.archive_issue(issue_id).await {
            Ok(success) => {
                if success {
                    self.loading = false;
                    self.mode = AppMode::Normal;
                    // Refresh issues to remove the deleted one
                    let _ = self.refresh_issues().await;
                    Ok(())
                } else {
                    self.loading = false;
                    self.error_message = Some("Failed to delete issue".to_string());
                    Err("Failed to delete issue".into())
                }
            }
            Err(e) => {
                self.loading = false;
                self.error_message = Some(format!("Failed to delete issue: {}", e));
                Err(e)
            }
        }
    }

    fn handle_confirm_mode_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
                // Toggle between Yes and No
                self.confirm_selection = !self.confirm_selection;
            }
            KeyCode::Enter => {
                if self.confirm_selection {
                    // User selected "Yes"
                    if let Some(action) = self.confirm_action.take() {
                        match action {
                            ConfirmAction::DeleteIssue(issue_id) => {
                                // Set the issue_id for deletion and switch to a temporary mode
                                // The actual deletion will be handled in the main loop
                                self.selected_issue_id = Some(issue_id);
                                // We need to trigger the deletion in the main loop
                                // For now, we'll use a special flag or handle it directly
                            }
                        }
                    }
                } else {
                    // User selected "No"
                    self.mode = AppMode::Normal;
                    self.confirm_action = None;
                    self.confirm_message.clear();
                    self.confirm_selection = false;
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(action) = self.confirm_action.take() {
                    match action {
                        ConfirmAction::DeleteIssue(issue_id) => {
                            // Set the issue_id for deletion and switch to a temporary mode
                            // The actual deletion will be handled in the main loop
                            self.selected_issue_id = Some(issue_id);
                            // We need to trigger the deletion in the main loop
                            // For now, we'll use a special flag or handle it directly
                        }
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.confirm_action = None;
                self.confirm_message.clear();
                self.confirm_selection = false;
            }
            _ => {}
        }
    }
}
