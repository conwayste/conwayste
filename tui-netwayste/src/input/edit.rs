use crate::app::App;

use crossterm::event::KeyCode;

pub fn handle_command_modification(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Down => {}
        KeyCode::Up => {}
        KeyCode::Enter => {
            // app.input_stage.next(),
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
        }
        KeyCode::Esc => {
            app.input_stage.prev();
        }
        _ => (),
    }
}

/*
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
*/
