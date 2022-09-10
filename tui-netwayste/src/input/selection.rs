use crate::app::{App, InputStage};
use crate::statefullist::StatefulList;

use crossterm::event::KeyCode;

pub fn handle_list_navigation(key: KeyCode, app: &mut App) {
    //pub fn handle_list_navigation<L: ToString>(key: KeyCode, list: &mut StatefulList<L>) {
    let list = app.displayed_menu_mut();
    let index = list.get_index();

    match key {
        KeyCode::Down => list.next(),
        KeyCode::Up => list.previous(),
        KeyCode::Char(c) => {
            if let Some(d) = c.to_digit(10) {
                let d = d as usize;
                if d <= list.items.len() {
                    list.select(d);
                }
            }
        }
        KeyCode::Enter => {
            if app.displayed_menu == 0 {
                app.displayed_menu = index + 1;
                app.edit_list_state = None;
            } else {
                // Already in a sub menu
                app.edit_index = Some(index + 1);
            }
            app.input_stage.next();
        }
        KeyCode::Esc => {
            app.displayed_menu = 0;
            app.edit_list_state = None;
            app.edit_index = None;
            app.input_stage = InputStage::SelectPacket;
        }
        _ => {}
    }
}
