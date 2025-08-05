use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::models::Issue;
use super::app::{AppMode, EditField, GroupBy, InteractiveApp};

#[derive(Debug)]
struct ColumnWidths {
    id: usize,
    priority: usize,
    title: usize,
    status: usize,
    assignee: usize,
    labels: usize,
    links: usize,
    // Visibility flags
    show_assignee: bool,
    show_labels: bool,
    show_links: bool,
    labels_as_count: bool,
}

fn calculate_column_widths(available_width: u16) -> ColumnWidths {
    let width = available_width as usize;
    
    // Minimum widths
    const MIN_ID: usize = 7;
    const MIN_PRIORITY: usize = 2;
    const MIN_TITLE: usize = 20;
    const MIN_STATUS: usize = 8;
    const MIN_ASSIGNEE: usize = 10;
    const MIN_LABELS: usize = 6;  // For count display "[2]"
    const MIN_LINKS: usize = 3;
    
    // Fixed widths
    let priority_width = 3; // 2 + space
    
    // Calculate based on terminal width
    if width < 80 {
        // Ultra narrow - only essentials
        ColumnWidths {
            id: MIN_ID,
            priority: priority_width,
            title: width.saturating_sub(MIN_ID + priority_width + MIN_STATUS + 4), // 4 for borders/padding
            status: MIN_STATUS,
            assignee: 0,
            labels: 0,
            links: 0,
            show_assignee: false,
            show_labels: false,
            show_links: false,
            labels_as_count: false,
        }
    } else if width < 100 {
        // Narrow - add assignee
        let essential_width = MIN_ID + priority_width + MIN_STATUS + MIN_ASSIGNEE + 5;
        ColumnWidths {
            id: MIN_ID,
            priority: priority_width,
            title: width.saturating_sub(essential_width),
            status: MIN_STATUS,
            assignee: MIN_ASSIGNEE,
            labels: 0,
            links: 0,
            show_assignee: true,
            show_labels: false,
            show_links: false,
            labels_as_count: false,
        }
    } else if width < 120 {
        // Medium - add labels as count
        let essential_width = MIN_ID + priority_width + MIN_STATUS + MIN_ASSIGNEE + MIN_LABELS + 6;
        ColumnWidths {
            id: 8,
            priority: priority_width,
            title: width.saturating_sub(essential_width),
            status: 10,
            assignee: 12,
            labels: MIN_LABELS,
            links: 0,
            show_assignee: true,
            show_labels: true,
            show_links: false,
            labels_as_count: true,
        }
    } else if width < 160 {
        // Wide - show labels normally
        let essential_width = MIN_ID + priority_width + 12 + 12 + 15 + MIN_LINKS + 7;
        ColumnWidths {
            id: 9,
            priority: priority_width,
            title: width.saturating_sub(essential_width),
            status: 12,
            assignee: 12,
            labels: 15,
            links: MIN_LINKS,
            show_assignee: true,
            show_labels: true,
            show_links: true,
            labels_as_count: false,
        }
    } else {
        // Extra wide - generous spacing
        let used_width = 10 + priority_width + 15 + 15 + 20 + 4 + 8;
        let remaining = width.saturating_sub(used_width);
        let title_width = remaining.min(80); // Cap title width for readability
        
        ColumnWidths {
            id: 10,
            priority: priority_width,
            title: title_width,
            status: 15,
            assignee: 15,
            labels: 20,
            links: 4,
            show_assignee: true,
            show_labels: true,
            show_links: true,
            labels_as_count: false,
        }
    }
}

pub fn draw(frame: &mut Frame, app: &InteractiveApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Footer
        ])
        .split(frame.size());

    draw_header(frame, chunks[0], app);
    
    match app.mode {
        AppMode::Detail | AppMode::Comment | AppMode::Edit | AppMode::EditField | AppMode::SelectOption | AppMode::ExternalEditor | AppMode::Links => {
            if let Some(issue) = app.get_selected_issue() {
                draw_issue_detail(frame, chunks[1], issue, app);
            }
        }
        _ => draw_issues_list(frame, chunks[1], app),
    }
    
    draw_footer(frame, chunks[2], app);
    
    // Draw overlays on top of everything
    match app.mode {
        AppMode::Comment => draw_comment_overlay(frame, frame.size(), &app.comment_input, app.comment_cursor_position),
        AppMode::Edit => draw_edit_menu_overlay(frame, frame.size(), app),
        AppMode::EditField => draw_edit_field_overlay(frame, frame.size(), app),
        AppMode::SelectOption => draw_select_option_overlay(frame, frame.size(), app),
        AppMode::ExternalEditor => {
            // Show a loading message while external editor is active
            let loading_area = centered_rect(50, 5, frame.size());
            frame.render_widget(Clear, loading_area);
            let loading_block = Block::default()
                .borders(Borders::ALL)
                .title(" External Editor ")
                .border_style(Style::default().fg(Color::Yellow));
            let loading_text = Paragraph::new("\nEditing in external editor...\nSave and exit to continue.")
                .block(loading_block)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow));
            frame.render_widget(loading_text, loading_area);
        }
        _ => {}
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &InteractiveApp) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(30)])
        .split(area);

    let title = match app.mode {
        AppMode::Normal => " Linear Interactive Mode ",
        AppMode::Search => " Search Mode ",
        AppMode::Filter => " Filter Mode ",
        AppMode::Detail => " Issue Detail ",
        AppMode::Comment => " Add Comment ",
        AppMode::Edit => " Edit Issue ",
        AppMode::EditField => " Edit Field ",
        AppMode::SelectOption => " Select Option ",
        AppMode::ExternalEditor => " External Editor ",
        AppMode::Links => " Navigate Links ",
    };

    let header = Paragraph::new(title)
        .style(Style::default().bg(Color::Black).fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(header, header_chunks[0]);

    let info = format!(" Issues: {} | Group by: {} ", 
        app.filtered_issues.len(),
        match app.group_by {
            GroupBy::Status => "Status",
            GroupBy::Project => "Project",
        }
    );
    let info_widget = Paragraph::new(info)
        .style(Style::default().bg(Color::Black).fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    frame.render_widget(info_widget, header_chunks[1]);
}

fn draw_issues_list(frame: &mut Frame, area: Rect, app: &InteractiveApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Issues ");

    if app.loading {
        let loading = Paragraph::new("Loading issues...")
            .style(Style::default().fg(Color::Yellow))
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(loading, area);
        return;
    }

    if let Some(error) = &app.error_message {
        let error_widget = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(error_widget, area);
        return;
    }

    if app.filtered_issues.is_empty() {
        let empty = Paragraph::new("No issues found")
            .style(Style::default().fg(Color::DarkGray))
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    // Calculate column widths based on available space
    let inner_width = area.width.saturating_sub(2); // Account for borders
    let col_widths = calculate_column_widths(inner_width);
    
    // Build dynamic header
    let header_style = Style::default().fg(Color::Gray).add_modifier(Modifier::UNDERLINED);
    let mut header = format!("{:<width$} {:<2}", "ID", "P", width = col_widths.id);
    header.push_str(&format!(" {:<width$}", "Title", width = col_widths.title));
    header.push_str(&format!(" {:<width$}", "Status", width = col_widths.status));
    
    if col_widths.show_assignee {
        header.push_str(&format!(" {:<width$}", "Assignee", width = col_widths.assignee));
    }
    if col_widths.show_labels {
        let label_header = if col_widths.labels_as_count { "Lbl" } else { "Labels" };
        header.push_str(&format!(" {:<width$}", label_header, width = col_widths.labels));
    }
    if col_widths.show_links {
        header.push_str(" 🔗");
    }
    
    let header_item = ListItem::new(header).style(header_style);
    
    let items: Vec<ListItem> = std::iter::once(header_item)
        .chain(app.filtered_issues
            .iter()
            .enumerate()
            .map(|(i, issue)| {
                let selected = i == app.selected_index;
                
                // Get priority symbol and color
                let (priority_symbol, priority_color) = match issue.priority {
                    Some(0) => (" ", Color::Gray),
                    Some(1) => ("◦", Color::Blue),
                    Some(2) => ("•", Color::Yellow),
                    Some(3) => ("■", Color::Rgb(255, 165, 0)), // Orange
                    Some(4) => ("▲", Color::Red),
                    _ => (" ", Color::Gray),
                };
                
                // Get status color based on state type
                let status_color = match issue.state.state_type.as_str() {
                    "backlog" => Color::Gray,
                    "unstarted" => Color::LightBlue,
                    "started" => Color::Yellow,
                    "completed" => Color::Green,
                    "canceled" => Color::DarkGray,
                    _ => Color::White,
                };
                
                // Format labels with colors
                let labels_str = if issue.labels.nodes.is_empty() {
                    String::new()
                } else {
                    issue.labels.nodes.iter()
                        .take(2)  // Show max 2 labels
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                
                let assignee_name = issue.assignee.as_ref()
                    .map(|a| parse_assignee_name(a))
                    .unwrap_or_else(|| "Unassigned".to_string());
                
                // Create styled spans for different parts
                // Build row with dynamic widths
                let id_span = ratatui::text::Span::styled(
                    format!("{:<width$}", truncate_id(&issue.identifier, col_widths.id), width = col_widths.id),
                    if selected { Style::default().bg(Color::DarkGray) } else { Style::default() }
                );
                
                let priority_span = ratatui::text::Span::styled(
                    format!(" {} ", priority_symbol),
                    if selected { 
                        Style::default().bg(Color::DarkGray).fg(priority_color) 
                    } else { 
                        Style::default().fg(priority_color) 
                    }
                );
                
                let title_span = ratatui::text::Span::styled(
                    format!("{:<width$}", truncate(&issue.title, col_widths.title), width = col_widths.title),
                    if selected { Style::default().bg(Color::DarkGray).fg(Color::White) } else { Style::default() }
                );
                
                let status_span = ratatui::text::Span::styled(
                    format!("{:<width$}", truncate(&issue.state.name, col_widths.status), width = col_widths.status),
                    if selected { 
                        Style::default().bg(Color::DarkGray).fg(status_color).add_modifier(Modifier::BOLD) 
                    } else { 
                        Style::default().fg(status_color) 
                    }
                );
                
                // Build dynamic row spans
                let mut spans = vec![id_span, priority_span, title_span, status_span];
                
                // Add optional columns
                if col_widths.show_assignee {
                    let assignee_span = ratatui::text::Span::styled(
                        format!("{:<width$}", truncate(&assignee_name, col_widths.assignee), width = col_widths.assignee),
                        if selected { Style::default().bg(Color::DarkGray).fg(Color::Cyan) } else { Style::default().fg(Color::Cyan) }
                    );
                    spans.push(assignee_span);
                }
                
                if col_widths.show_labels {
                    let labels_display = if col_widths.labels_as_count {
                        // Show label count
                        if issue.labels.nodes.is_empty() {
                            "   ".to_string()
                        } else {
                            format!("[{}]", issue.labels.nodes.len())
                        }
                    } else {
                        // Show label names
                        labels_str.clone()
                    };
                    
                    let labels_span = ratatui::text::Span::styled(
                        format!("{:<width$}", truncate(&labels_display, col_widths.labels), width = col_widths.labels),
                        if selected { Style::default().bg(Color::DarkGray).fg(Color::Magenta) } else { Style::default().fg(Color::Magenta) }
                    );
                    spans.push(labels_span);
                }
                
                if col_widths.show_links {
                    // Get links count (excluding the Linear URL itself)
                    let links = get_issue_links(issue);
                    let extra_links_count = if links.len() > 1 { links.len() - 1 } else { 0 };
                    let links_text = if extra_links_count > 0 {
                        format!(" {} ", extra_links_count)
                    } else {
                        "   ".to_string()
                    };
                    
                    let links_span = ratatui::text::Span::styled(
                        links_text,
                        if selected { 
                            Style::default().bg(Color::DarkGray).fg(Color::Blue) 
                        } else { 
                            Style::default().fg(Color::Blue) 
                        }
                    );
                    spans.push(links_span);
                }
                
                let line = ratatui::text::Line::from(spans);
                ListItem::new(line)
            }))
        .collect();

    let list = List::new(items)
        .block(block)
        .style(Style::default().fg(Color::White));
    
    frame.render_widget(list, area);

    // Draw search overlay if in search mode
    if app.mode == AppMode::Search {
        draw_search_overlay(frame, area, &app.search_query);
    }
    
    // Draw comment overlay if in comment mode
    if app.mode == AppMode::Comment {
        draw_comment_overlay(frame, area, &app.comment_input, app.comment_cursor_position);
    }
}

fn draw_issue_detail(frame: &mut Frame, area: Rect, issue: &Issue, app: &InteractiveApp) {
    let links = get_issue_links(issue);
    let has_links = links.len() > 1; // More than just the Linear URL
    
    let constraints = if has_links {
        // Limit links section to max 12 lines (header + 10 links + scroll indicator)
        let links_height = (3 + links.len() as u16).min(12);
        vec![
            Constraint::Length(4),   // Title
            Constraint::Length(3),   // Metadata
            Constraint::Min(10),     // Description
            Constraint::Length(links_height), // Links section with max height
        ]
    } else {
        vec![
            Constraint::Length(4),   // Title
            Constraint::Length(3),   // Metadata
            Constraint::Min(10),     // Description
        ]
    };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title(" Issue ");
    let title = Paragraph::new(format!("{} - {}", issue.identifier, issue.title))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(title_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(title, chunks[0]);

    // Metadata with colored elements
    let status_color = match issue.state.state_type.as_str() {
        "backlog" => Color::Gray,
        "unstarted" => Color::LightBlue,
        "started" => Color::Yellow,
        "completed" => Color::Green,
        "canceled" => Color::DarkGray,
        _ => Color::White,
    };
    
    let (priority_name, priority_color) = match issue.priority {
        Some(0) => ("None", Color::Gray),
        Some(1) => ("Low", Color::Blue),
        Some(2) => ("Medium", Color::Yellow),
        Some(3) => ("High", Color::Rgb(255, 165, 0)),
        Some(4) => ("Urgent", Color::Red),
        _ => ("Unknown", Color::Gray),
    };
    
    let mut metadata_spans = vec![
        Span::raw("State: "),
        Span::styled(&issue.state.name, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::raw(" | Assignee: "),
        Span::styled(
            issue.assignee.as_ref()
                .map(|a| parse_assignee_name(a))
                .unwrap_or_else(|| "Unassigned".to_string()),
            Style::default().fg(Color::Cyan)
        ),
        Span::raw(" | Team: "),
        Span::styled(&issue.team.name, Style::default().fg(Color::LightGreen)),
        Span::raw(" | Priority: "),
        Span::styled(priority_name, Style::default().fg(priority_color).add_modifier(Modifier::BOLD)),
    ];
    
    if !issue.labels.nodes.is_empty() {
        metadata_spans.push(Span::raw(" | Labels: "));
        for (i, label) in issue.labels.nodes.iter().enumerate() {
            if i > 0 {
                metadata_spans.push(Span::raw(", "));
            }
            metadata_spans.push(Span::styled(&label.name, Style::default().fg(Color::Magenta)));
        }
    }
    
    let metadata_line = Line::from(metadata_spans);
    let metadata_widget = Paragraph::new(vec![metadata_line])
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(metadata_widget, chunks[1]);

    // Description
    let description = issue.description.as_deref().unwrap_or("No description");
    let desc_widget = Paragraph::new(description)
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL).title(" Description "))
        .wrap(Wrap { trim: true });
    frame.render_widget(desc_widget, chunks[2]);
    
    // Links section (if there are links beyond the Linear URL)
    if has_links {
        let mut link_lines = vec![];
        
        // Calculate available height for links (subtract 2 for header, 1 for border)
        let available_height = chunks[3].height.saturating_sub(3) as usize;
        let max_visible_links = available_height.saturating_sub(1); // Reserve space for navigation help
        
        if app.mode == AppMode::Links {
            link_lines.push(Line::from(Span::styled("Navigate with j/k or ↑/↓, Enter to open, Esc to exit", Style::default().fg(Color::Gray))));
        } else {
            link_lines.push(Line::from(Span::styled("Press 'l' to navigate links, 'o' for Linear, or number keys:", Style::default().fg(Color::Gray))));
        }
        link_lines.push(Line::from(""));
        
        // Calculate visible range with scrolling
        let selected_idx = if app.mode == AppMode::Links { app.selected_link_index } else { 0 };
        let half_visible = max_visible_links / 2;
        
        let (start_idx, end_idx) = if links.len() <= max_visible_links {
            // All links fit
            (0, links.len())
        } else {
            // Need scrolling
            let start = if selected_idx < half_visible {
                0
            } else if selected_idx > links.len() - half_visible {
                links.len().saturating_sub(max_visible_links)
            } else {
                selected_idx.saturating_sub(half_visible)
            };
            (start, (start + max_visible_links).min(links.len()))
        };
        
        // Add scroll indicator at top
        if start_idx > 0 {
            link_lines.push(Line::from(Span::styled(
                format!("    ↑ {} more", start_idx),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            )));
        }
        
        // Show visible links
        for i in start_idx..end_idx {
            let link = &links[i];
            let link_text = if i == 0 {
                format!("[o] Linear: {}", truncate(link, 60))
            } else if i < 10 {
                format!("[{}] {}", i, truncate(link, 60))
            } else {
                format!("    {}", truncate(link, 60))
            };
            
            let is_selected = app.mode == AppMode::Links && i == app.selected_link_index;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(Color::White).add_modifier(Modifier::BOLD)
            } else if i == 0 {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Blue)
            };
            
            link_lines.push(Line::from(Span::styled(link_text, style)));
        }
        
        // Add scroll indicator at bottom
        if end_idx < links.len() {
            link_lines.push(Line::from(Span::styled(
                format!("    ↓ {} more", links.len() - end_idx),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
            )));
        }
        
        let border_style = if app.mode == AppMode::Links {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        let title = if links.len() > max_visible_links && app.mode == AppMode::Links {
            format!(" Links ({}/{}) ", selected_idx + 1, links.len())
        } else {
            " Links ".to_string()
        };
        
        let links_widget = Paragraph::new(link_lines)
            .block(Block::default().borders(Borders::ALL).title(title).border_style(border_style));
        frame.render_widget(links_widget, chunks[3]);
    }
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &InteractiveApp) {
    let help_text = match app.mode {
        AppMode::Normal => {
            "[q] Quit  [j/k] Nav  [Enter] View  [e] Edit  [o] Open  [/] Search  [g] Group  [r] Refresh"
        }
        AppMode::Search => {
            "[Esc] Cancel  [Enter] Apply  Type to search..."
        }
        AppMode::Filter => {
            "[Esc] Back  [Enter] Apply Filter"
        }
        AppMode::Detail => {
            "[Esc/q] Back  [e] Edit  [c] Comment  [o] Open Linear  [l] Navigate links  [0-9] Quick open"
        }
        AppMode::Comment => {
            "[Esc] Cancel  [Enter] Submit  Type your comment..."
        }
        AppMode::Edit => {
            "[↑/↓] Select Field  [Enter] Edit  [Esc] Cancel"
        }
        AppMode::EditField => {
            if let EditField::Description = app.edit_field {
                "[Enter] Save  [Esc] Cancel  [Ctrl+E] External Editor  [←/→] Move cursor"
            } else {
                "[Enter] Save  [Esc] Cancel  [←/→] Move cursor  Type to edit..."
            }
        }
        AppMode::SelectOption => {
            "[↑/↓] Select  [Enter] Confirm  [Esc/q] Cancel"
        }
        AppMode::ExternalEditor => {
            "Launching external editor..."
        }
        AppMode::Links => {
            "[j/k or ↑/↓] Navigate  [Enter/o] Open link  [Esc/q] Back"
        }
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().bg(Color::Black).fg(Color::Green))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .alignment(Alignment::Center);
    frame.render_widget(footer, area);
}

fn draw_search_overlay(frame: &mut Frame, area: Rect, search_query: &str) {
    let popup_area = centered_rect(60, 3, area);
    
    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(" Search ")
        .style(Style::default().bg(Color::Black));
    
    let search_text = Paragraph::new(format!("Search: {}_", search_query))
        .style(Style::default().fg(Color::Yellow))
        .block(search_block);
    
    frame.render_widget(search_text, popup_area);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height - height) / 2),
            Constraint::Length(height),
            Constraint::Length((area.height - height) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_comment_overlay(frame: &mut Frame, area: Rect, comment_input: &str, cursor_position: usize) {
    let popup_area = centered_rect(70, 10, area);
    
    // First, clear the area completely
    frame.render_widget(Clear, popup_area);
    
    // Draw a shadow/border effect around the popup
    let shadow_area = Rect {
        x: popup_area.x.saturating_sub(1),
        y: popup_area.y.saturating_sub(1),
        width: popup_area.width + 2,
        height: popup_area.height + 2,
    };
    let shadow = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(shadow, shadow_area);
    
    // Now draw the main comment box
    let comment_block = Block::default()
        .borders(Borders::ALL)
        .title("╭─ Add Comment ─╮")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Yellow).bg(Color::Black).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Black));
    
    frame.render_widget(comment_block.clone(), popup_area);
    
    let inner_area = comment_block.inner(popup_area);
    
    // Add some padding
    let text_area = Rect {
        x: inner_area.x + 1,
        y: inner_area.y + 1,
        width: inner_area.width.saturating_sub(2),
        height: inner_area.height.saturating_sub(2),
    };
    
    if comment_input.is_empty() {
        let help_text = vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("Type your comment below:").style(Style::default().fg(Color::Gray)),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("_").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK)),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("[Enter] Submit • [Esc] Cancel • [←/→] Move cursor").style(Style::default().fg(Color::DarkGray)),
        ];
        let help_paragraph = Paragraph::new(help_text)
            .alignment(Alignment::Center);
        frame.render_widget(help_paragraph, text_area);
    } else {
        // Create the text with cursor
        let (before_cursor, after_cursor) = comment_input.split_at(cursor_position);
        let mut spans = vec![
            ratatui::text::Span::raw(before_cursor),
            ratatui::text::Span::styled("_", Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK)),
        ];
        if !after_cursor.is_empty() {
            spans.push(ratatui::text::Span::raw(after_cursor));
        }
        
        let input_text = vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(spans),
        ];
        let input_paragraph = Paragraph::new(input_text)
            .wrap(Wrap { trim: true });
        frame.render_widget(input_paragraph, text_area);
        
        // Show help at bottom
        let help_area = Rect {
            x: text_area.x,
            y: text_area.y + text_area.height.saturating_sub(1),
            width: text_area.width,
            height: 1,
        };
        let help = Paragraph::new("[Enter] Submit • [Esc] Cancel • [←/→] Move cursor")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(help, help_area);
    }
}

fn draw_edit_menu_overlay(frame: &mut Frame, area: Rect, app: &InteractiveApp) {
    let popup_area = centered_rect(60, 12, area);
    
    // Clear the area
    frame.render_widget(Clear, popup_area);
    
    // Draw shadow
    let shadow_area = Rect {
        x: popup_area.x.saturating_sub(1),
        y: popup_area.y.saturating_sub(1),
        width: popup_area.width + 2,
        height: popup_area.height + 2,
    };
    let shadow = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(shadow, shadow_area);
    
    // Draw main box
    let edit_block = Block::default()
        .borders(Borders::ALL)
        .title("╭─ Edit Issue ─╮")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan).bg(Color::Black).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Black));
    
    frame.render_widget(edit_block.clone(), popup_area);
    
    let inner_area = edit_block.inner(popup_area);
    
    // Create menu items
    let fields = vec![
        ("Title", 0),
        ("Description", 1),
        ("Status", 2),
        ("Assignee", 3),
        ("Priority", 4),
    ];
    
    let mut lines = vec![ratatui::text::Line::from("")];
    
    for (name, index) in fields {
        let style = if index == app.edit_field_index {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        
        let prefix = if index == app.edit_field_index { " › " } else { "   " };
        let suffix = match (name, index) {
            ("Status", _) | ("Priority", _) | ("Assignee", _) => " [select]",
            ("Description", _) => " [Enter or E for editor]",
            _ => "",
        };
        
        lines.push(ratatui::text::Line::from(format!("{}{}{}", prefix, name, suffix)).style(style));
    }
    
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from("Use ↑/↓ to select, Enter to edit").style(Style::default().fg(Color::DarkGray)));
    
    let menu = Paragraph::new(lines);
    frame.render_widget(menu, inner_area);
}

fn draw_edit_field_overlay(frame: &mut Frame, area: Rect, app: &InteractiveApp) {
    let popup_area = centered_rect(70, 10, area);
    
    // Clear the area
    frame.render_widget(Clear, popup_area);
    
    // Draw shadow
    let shadow_area = Rect {
        x: popup_area.x.saturating_sub(1),
        y: popup_area.y.saturating_sub(1),
        width: popup_area.width + 2,
        height: popup_area.height + 2,
    };
    let shadow = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(shadow, shadow_area);
    
    // Draw main box
    let field_name = match app.edit_field {
        EditField::Title => "Title",
        EditField::Description => "Description",
        EditField::Status => "Status",
        EditField::Assignee => "Assignee",
        EditField::Priority => "Priority",
    };
    
    let edit_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("╭─ Edit {} ─╮", field_name))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Green).bg(Color::Black).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Black));
    
    frame.render_widget(edit_block.clone(), popup_area);
    
    let inner_area = edit_block.inner(popup_area);
    let text_area = Rect {
        x: inner_area.x + 1,
        y: inner_area.y + 1,
        width: inner_area.width.saturating_sub(2),
        height: inner_area.height.saturating_sub(2),
    };
    
    let input_text = if app.edit_input.is_empty() {
        vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(format!("Current value: (empty)")).style(Style::default().fg(Color::DarkGray)),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("_").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK)),
        ]
    } else {
        // Create the text with cursor
        let (before_cursor, after_cursor) = app.edit_input.split_at(app.cursor_position);
        let mut spans = vec![
            ratatui::text::Span::raw(before_cursor),
            ratatui::text::Span::styled("_", Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK)),
        ];
        if !after_cursor.is_empty() {
            spans.push(ratatui::text::Span::raw(after_cursor));
        }
        
        vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(spans),
        ]
    };
    
    let input_paragraph = Paragraph::new(input_text)
        .wrap(Wrap { trim: true });
    frame.render_widget(input_paragraph, text_area);
    
    // Show help at bottom
    let help_area = Rect {
        x: text_area.x,
        y: text_area.y + text_area.height.saturating_sub(1),
        width: text_area.width,
        height: 1,
    };
    let help = Paragraph::new("[Enter] Save • [Esc] Cancel • [←/→] Move cursor")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, help_area);
}

fn draw_select_option_overlay(frame: &mut Frame, area: Rect, app: &InteractiveApp) {
    let height = match app.edit_field {
        EditField::Status => (app.workflow_states.len() + 4).min(20) as u16,
        EditField::Priority => 9,
        _ => 10,
    };
    
    let popup_area = centered_rect(60, height, area);
    
    // Clear the area
    frame.render_widget(Clear, popup_area);
    
    // Draw shadow
    let shadow_area = Rect {
        x: popup_area.x.saturating_sub(1),
        y: popup_area.y.saturating_sub(1),
        width: popup_area.width + 2,
        height: popup_area.height + 2,
    };
    let shadow = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(shadow, shadow_area);
    
    // Draw main box
    let title = match app.edit_field {
        EditField::Status => "Select Status",
        EditField::Priority => "Select Priority",
        _ => "Select Option",
    };
    
    let select_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("╭─ {} ─╮", title))
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Magenta).bg(Color::Black).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Black));
    
    frame.render_widget(select_block.clone(), popup_area);
    
    let inner_area = select_block.inner(popup_area);
    
    // Create list items based on field type
    let items: Vec<ListItem> = match app.edit_field {
        EditField::Status => {
            if app.workflow_states.is_empty() {
                vec![ListItem::new(" No workflow states available ").style(Style::default().fg(Color::Red))]
            } else {
                app.workflow_states
                    .iter()
                    .enumerate()
                    .map(|(i, state)| {
                        let current_marker = if let Some(issue) = app.get_selected_issue() {
                            if issue.state.name == state.name { " (current)" } else { "" }
                        } else {
                            ""
                        };
                        let content = format!(" {}{} ", state.name, current_marker);
                        let style = if i == app.option_index {
                            Style::default().fg(Color::Black).bg(Color::Magenta)
                        } else if !current_marker.is_empty() {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect()
            }
        }
        EditField::Priority => {
            let priorities = vec![
                ("None", 0),
                ("Low", 1),
                ("Medium", 2),
                ("High", 3),
                ("Urgent", 4),
            ];
            
            priorities
                .iter()
                .enumerate()
                .map(|(i, (name, _))| {
                    let content = format!(" {} ", name);
                    let style = if i == app.option_index {
                        Style::default().fg(Color::Black).bg(Color::Magenta)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(content).style(style)
                })
                .collect()
        }
        _ => vec![],
    };
    
    let list = List::new(items);
    frame.render_widget(list, inner_area);
}

fn truncate(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else {
        format!("{}...", &s[..max_width - 3])
    }
}

fn truncate_id(id: &str, max_width: usize) -> String {
    if id.len() <= max_width {
        id.to_string()
    } else {
        // Try to extract just the number part for very narrow displays
        if let Some(dash_pos) = id.find('-') {
            let number_part = &id[dash_pos + 1..];
            if number_part.len() <= max_width {
                return number_part.to_string();
            }
        }
        truncate(id, max_width)
    }
}

fn parse_assignee_name(user: &crate::models::User) -> String {
    // First try to extract username from email
    if let Some(username) = user.email.split('@').next() {
        if !username.is_empty() {
            return username.to_string();
        }
    }
    
    // Otherwise, try to get first name
    if let Some(first_name) = user.name.split_whitespace().next() {
        if !first_name.is_empty() {
            return first_name.to_string();
        }
    }
    
    // Fallback to full name
    user.name.clone()
}

fn extract_links_from_text(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    
    // Match URLs (http/https)
    let url_regex = regex::Regex::new(r#"https?://[^\s<>"{}|\\^`\[\]]+"#).unwrap();
    for capture in url_regex.captures_iter(text) {
        links.push(capture[0].to_string());
    }
    
    // Match markdown links [text](url)
    let md_link_regex = regex::Regex::new(r#"\[([^\]]+)\]\(([^)]+)\)"#).unwrap();
    for capture in md_link_regex.captures_iter(text) {
        if let Some(url) = capture.get(2) {
            links.push(url.as_str().to_string());
        }
    }
    
    links
}

pub fn get_issue_links(issue: &crate::models::Issue) -> Vec<String> {
    let mut all_links = vec![issue.url.clone()]; // Always include the Linear URL
    
    if let Some(desc) = &issue.description {
        all_links.extend(extract_links_from_text(desc));
    }
    
    // Deduplicate
    all_links.sort();
    all_links.dedup();
    all_links
}