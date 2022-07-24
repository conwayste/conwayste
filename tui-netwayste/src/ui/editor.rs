use crate::app::App;

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

fn editor_ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
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

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .displayed_editor
        .items
        .iter()
        .map(|field| {
            let lines = vec![Spans::from(format!("{} -> {}", field.name, field.value))];
            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let editing_cmd_name = app.displayed_menu.items[app.displayed_menu.state.selected().unwrap()].to_string();
    let items = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    if app.editing {
        let index = app.displayed_editor.state.selected().unwrap();
        let label_text = &app.displayed_editor.items[index].name;
        let editing_text = &app.displayed_editor.items[index].value;
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

    // We can now render the item list
    f.render_stateful_widget(items, area, &mut app.displayed_editor.state);
}
