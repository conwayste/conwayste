use crate::app::App;
use crate::statefullist::StatefulList;

use crossterm::event::KeyCode;

pub fn handle_list_navigation<L: ToString>(key: KeyCode, list: &mut StatefulList<L>) {
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
        }
        KeyCode::Esc => {
        }
        _ => {}
    }
}