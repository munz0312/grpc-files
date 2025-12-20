use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::{fileservice::FileInfo, tui::app::App};

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}

pub fn ui(frame: &mut Frame, app: &App) {
    // If we're in uploading mode, show a full-screen message
    if app.is_uploading() {
        let area = centered_rect(60, 20, frame.area());
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Selecting File for Upload",
                Style::default().fg(Color::Yellow).bold(),
            )),
            Line::from(""),
            Line::from("The file selection dialog will open in a separate window."),
            Line::from(""),
            Line::from("Please select a file to upload..."),
            Line::from(""),
        ];
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Upload Mode"))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        frame.render_widget(Clear, frame.area()); // Clear the entire frame
        frame.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(frame.area());

    // Calculate column widths based on terminal width
    let list_width = chunks[0].width;
    // Reserve space for arrows and borders: 2 for arrows/bullets + 2 for borders + 4 for spacing = 8
    // Size column gets 10 chars, timestamp gets 19 chars (YYYY-MM-DD HH:MM:SS)
    let filename_width = (list_width as usize).saturating_sub(8 + 10 + 19);
    let filename_width = filename_width.max(10); // Minimum filename width

    let items: Vec<ListItem> = app
        .files()
        .iter()
        .enumerate()
        .map(|(i, file_info)| {
            let (filename, size, upload_time) = format_file_info(file_info);

            // Truncate filename if too long and add ellipsis
            let display_filename = if filename.len() > filename_width {
                format!("{}...", &filename[..filename_width.saturating_sub(3)])
            } else {
                filename
            };

            // Pad filename to use full allocated width
            let padded_filename = format!("{:<width$}", display_filename, width = filename_width);

            // Fixed width for size column (10 chars for alignment)
            let padded_size = format!("{:>10}", size);

            // Fixed width for timestamp (19 chars for YYYY-MM-DD HH:MM:SS)
            let padded_time = format!("{:<19}", upload_time);

            let line = if i == app.selected_index() {
                Line::from(vec![
                    Span::styled("→ ", Style::default().fg(Color::Yellow)),
                    Span::styled(padded_filename, Style::default().fg(Color::Yellow).bold()),
                    Span::raw(" "),
                    Span::styled(padded_size, Style::default().fg(Color::Yellow)),
                    Span::raw(" "),
                    Span::styled(padded_time, Style::default().fg(Color::Yellow)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(padded_filename, Style::default()),
                    Span::raw(" "),
                    Span::styled(padded_size, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(padded_time, Style::default().fg(Color::DarkGray)),
                ])
            };
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" File Server Browser ")
                .title_bottom(" r: refresh | d: download | X: delete | U: upload | q: quit "),
        )
        .highlight_style(Style::default());

    frame.render_widget(list, chunks[0]);
    // Status bar
    let status_text = if let Some(msg) = app.status_message() {
        msg.clone()
    } else if app.files().is_empty() {
        "No files. Press r to refresh".to_string()
    } else {
        format!(
            "Selected: {}",
            app.files()
                .get(app.selected_index())
                .map(|f| &f.filename)
                .unwrap_or(&"".to_string())
        )
    };

    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: true });

    frame.render_widget(status, chunks[1]);
}

fn format_file_info(file_info: &FileInfo) -> (String, String, String) {
    let filename = file_info.filename.clone();

    // Format file size
    let size = format_bytes(file_info.size);

    // Format upload time
    let upload_time = if let Some(timestamp) = &file_info.upload_time {
        let upload_str = timestamp.to_string();
        let parts: Vec<&str> = upload_str.split('T').collect();
        if parts.len() >= 2 {
            let date = parts[0];
            let time = parts[1].split('.').next().unwrap_or("00:00:00");
            format!("{} {}", date, time)
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    (filename, size, upload_time)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
