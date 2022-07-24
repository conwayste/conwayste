use crate::app::App;

use crossterm::event::KeyCode;

pub fn handle_list_navigation(key: KeyCode, app: &mut App) {
    match key {
        KeyCode::Down => app.displayed_menu.next(),
        KeyCode::Up => app.displayed_menu.previous(),
        KeyCode::Char(c) => {
            if let Some(d) = c.to_digit(10) {
                let d = d as usize;
                if d <= app.displayed_menu.items.len() {
                    app.displayed_menu.select(d);
                }
            }
        }
        KeyCode::Enter => {
        }
        KeyCode::Esc => {
        }
        _ => {}
    }
}