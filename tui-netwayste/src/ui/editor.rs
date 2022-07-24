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

fn handle_navigateedit_keys(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Down => app.displayed_editor.next(),
        KeyCode::Up => app.displayed_editor.previous(),
        KeyCode::Enter => app.input_stage.next(),
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if !app.editing {
                app.editing = true;
                let index = app.displayed_editor.state.selected().unwrap();
                app.preedit_text = app.displayed_editor.items[index].value.clone();
            }
        }
        KeyCode::Esc => {
            let mut opt_edit_cmd = None;
            if let Some(index) = app.displayed_menu.state.selected() {
                let item_name = &app.displayed_menu.items[index];
                if let Some(entry) = app.menu_item_map.get_mut(item_name) {
                    match entry {
                        MenuItemEntry::EditDialog(edit_cmd) => opt_edit_cmd = Some(edit_cmd),
                        _ => unreachable!("It should be impossible to be editing anything other than the EditDialog"),
                    }
                }
            }

            // Unwrap okay because all other paths are unreachable
            let edit_cmd_mut_ref = opt_edit_cmd.unwrap();

            // Save the editing window state
            edit_cmd_mut_ref.fields = app.displayed_editor.clone();
            app.input_stage.prev();
        }
        _ => (),
    }
}

fn handle_editcommand_keys(key: KeyCode, app: &mut App) {
    let field_value = &mut app.displayed_editor.items[app.displayed_editor.state.selected().unwrap()].value;

    match key {
        KeyCode::Char(ch) => field_value.push(ch),
        KeyCode::Backspace => {
            field_value.pop();
        }
        KeyCode::Delete => field_value.clear(),
        KeyCode::Enter => {
            app.editing = false;
            app.preedit_text.clear();
        }
        KeyCode::Esc => {
            // Abort editing
            app.editing = false;

            let index = app.displayed_editor.state.selected().unwrap();
        }
        _ => (),
    }
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