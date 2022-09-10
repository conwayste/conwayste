// All credit goes to the tui-rs developers
// Some modifications have been made for private use

use std::vec;

use tui::widgets::ListState;

#[derive(Clone)]
pub struct FieldEditList {
    pub state: ListState,
    pub field_input_buffer: String,
    pub fields: Vec<String>,
    pub values: Vec<String>,
}

impl FieldEditList {
    pub fn with_fields(fields: Vec<String>) -> FieldEditList {
        let mut values = vec![];
        for _ in 0..fields.len() {
            values.push(String::new());
        }
        let mut field_edit_list = FieldEditList {
            state: ListState::default(),
            fields,
            values,
            field_input_buffer: String::new(),
        };
        field_edit_list.state.select(Some(0));
        field_edit_list
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.fields.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.fields.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn select(&mut self, idx: usize) {
        self.state.select(Some(idx));
        if let Some(string) = self.values.get(idx) {
            self.field_input_buffer = string.to_string();
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn get_index(&self) -> usize {
        if let Some(x) = self.state.selected() {
            x
        } else {
            0
        }
    }

    pub fn update_field(&mut self, input: String) {
        let index = self.get_index();

    }

    pub fn save(&mut self) {
        let index = self.get_index();
        if let Some(string) = self.values.get_mut(index) {
            *string = self.field_input_buffer.to_string();
        }
    }
}
