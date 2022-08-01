use crate::app::App;
use crate::nw::get_mimic_meta_from;

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw_edit_ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();

    let msg = if !app.editing {
        vec![
            Span::raw("Press "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD).bg(Color::Red)),
            Span::raw(" to cancel | Select with "),
            Span::styled(
                "Up/Down",
                Style::default().add_modifier(Modifier::BOLD).bg(Color::Yellow),
            ),
            Span::raw(" | Edit with "),
            Span::styled("E", Style::default().add_modifier(Modifier::BOLD).bg(Color::Magenta)),
            Span::raw(" | "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD).bg(Color::Green)),
            Span::raw(" to transmit"),
        ]
    } else {
        vec![
            Span::raw("Press "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD).bg(Color::Red)),
            Span::raw(" to cancel | Confirm with "),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD).bg(Color::Green)),
            Span::raw(" | Delete with "),
            Span::styled(
                "Backspace",
                Style::default().add_modifier(Modifier::BOLD).bg(Color::Magenta),
            ),
            Span::raw(" | Clear with "),
            Span::styled(
                "Delete",
                Style::default().add_modifier(Modifier::BOLD).bg(Color::Yellow),
            ),
        ]
    };

    let title = Spans::from(msg);

    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray))
        .title(title)
        .title_alignment(Alignment::Center);

    let area = centered_rect(80, 20, size);
    f.render_widget(Clear, area); //this clears out the background

    let command_index = app.displayed_menu_mut().get_index();

    let mimic_metadata = get_mimic_meta_from(&app.ra_data[command_index]);

    // Iterate through all elements in the `items` app and append some debug text to it.
    let mut items: Vec<ListItem> = vec![];
    if let Some(metadata) = mimic_metadata {
        // FIXME: MetadataField needs to be bound to Iterator
        for field in &metadata.fields {
            let lines = vec![Spans::from(format!(
                "{} ({}) {}",
                field.name,
                field.type_,
                "PLACEHOLDER".to_owned()
            ))];
            items.push(ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White)));
        }
    }

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    if app.editing {
        let index = 0;
        let label_text = "label_text Fixme!";
        let editing_text = "editing_text Fixme!";
        f.set_cursor(
            area.x
                + ">> ".width() as u16
                + label_text.width() as u16
                + " -> ".width() as u16
                + editing_text.width() as u16
                + 1,
            area.y + index as u16 + 1,
        );
    }

    let mut list_state = ListState::default();
    // We can now render the item list
    f.render_stateful_widget(items, area, &mut list_state);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
