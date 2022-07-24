use crate::app::App;

use crossterm::event::KeyCode;

fn handle_command_edit(key: KeyCode, app: &mut App) {
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