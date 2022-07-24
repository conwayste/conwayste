use crate::statefullist::StatefulList;
use netwaystev2::filter::FilterMode;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Field {
    name:  String,
    value: String,
}

impl Field {
    fn new(name: &str, value: &str) -> Self {
        Field {
            name:  name.into(),
            value: value.into(),
        }
    }
}

pub struct EditableCommand {
    fields: StatefulList<Field>,
}

impl EditableCommand {
    fn with_items(list_fields: Vec<Field>) -> Self {
        EditableCommand {
            fields: StatefulList::with_items(list_fields),
        }
    }
}

pub enum MenuItemEntry {
    MenuIndex(usize),
    EditDialog(EditableCommand),
}

/// This struct holds the current state of the app. In particular, it has the `items` field which is a wrapper
/// around `ListState`. Keeping track of the items state let us render the associated widget with its state
/// and have access to features such as natural scrolling.
///
/// Check the event handling at the bottom to see how to change the state on incoming events.
/// Check the drawing logic for items on how to specify the highlighting style for selected items.
pub struct App<'a> {
    pub mode:               FilterMode,
    pub input_stage:        InputStage,
    pub editing:            bool,   // Are we editing a field?
    pub preedit_text:       String, // Previous field value while editing it; restored on cancel
    pub displayed_menu:     StatefulList<String>,
    pub menus:              Vec<StatefulList<String>>,
    pub menu_item_map:      HashMap<String, MenuItemEntry>,
    pub events:             Vec<(&'a str, &'a str)>,
}

impl<'a> App<'a> {
    pub(crate) fn new() -> App<'a> {
        let mut menu_item_map = HashMap::new();
        menu_item_map.insert("RequestAction".to_owned(), MenuItemEntry::MenuIndex(1));
        menu_item_map.insert("ResponseCode".to_owned(), MenuItemEntry::MenuIndex(2));

        let menus = vec![StatefulList::with_items(vec![
            "RequestAction".to_owned(),
            "ResponseCode".to_owned(),
        ])];

        let menu_display_index = 0;
        let displayed_menu = menus[menu_display_index].clone();

        App {
            mode: FilterMode::Client,
            input_stage: InputStage::CommandSelection,
            editing: false,
            preedit_text: String::new(),
            displayed_menu,
            menus,
            menu_item_map,
            events: vec![
                ("Event1", "INFO"),
                ("Event2", "INFO"),
                ("Event3", "CRITICAL"),
                ("Event4", "ERROR"),
                ("Event5", "INFO"),
                ("Event6", "INFO"),
                ("Event7", "WARNING"),
                ("Event8", "INFO"),
                ("Event9", "INFO"),
                ("Event10", "INFO"),
                ("Event11", "CRITICAL"),
                ("Event12", "INFO"),
                ("Event13", "INFO"),
                ("Event14", "INFO"),
                ("Event15", "INFO"),
                ("Event16", "INFO"),
                ("Event17", "ERROR"),
                ("Event18", "ERROR"),
                ("Event19", "INFO"),
                ("Event20", "INFO"),
                ("Event21", "WARNING"),
                ("Event22", "INFO"),
                ("Event23", "INFO"),
                ("Event24", "WARNING"),
                ("Event25", "INFO"),
                ("Event26", "INFO"),
            ],
        }
    }

    /// Rotate through the event list.
    /// This only exists to simulate some kind of "progress"
    pub fn on_tick(&mut self) {
        let event = self.events.remove(0);
        self.events.push(event);
    }
}

#[derive(PartialEq)]
pub enum InputStage {
    CommandSelection,
    CommandModification,
    SendCommand,
}

impl InputStage {
    pub fn next(&mut self) {
        *self = match self {
            InputStage::CommandSelection => InputStage::CommandModification,
            InputStage::CommandModification => InputStage::SendCommand,
            InputStage::SendCommand => InputStage::CommandSelection,
        };
    }

    pub fn prev(&mut self) {
        *self = match self {
            InputStage::CommandSelection => InputStage::CommandSelection,
            InputStage::CommandModification => InputStage::CommandSelection,
            InputStage::SendCommand => InputStage::CommandModification,
        };
    }
}
