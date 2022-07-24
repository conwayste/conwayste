use std::vec;

use crate::statefullist::StatefulList;
use netwaystev2::filter::FilterMode;

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
    pub displayed_menu:     usize,
    pub edit_index : Option<usize>,
    pub menus:              Vec<StatefulList<String>>,
    pub events:             Vec<(&'a str, &'a str)>,
}

impl<'a> App<'a> {
    pub(crate) fn new() -> App<'a> {
        let root = StatefulList::with_items(vec![
                "RequestAction".to_owned(),
                "ResponseCode".to_owned(),
            ]);

        let ra_list = StatefulList::with_items(vec![
            "RA_one".to_owned(),
            "RA_two".to_owned(),
        ]);

        let rc_list = StatefulList::with_items(vec![
            "RC_one".to_owned(),
            "RC_two".to_owned(),
        ]);

        let menus = vec![
            root, ra_list, rc_list
        ];

        App {
            mode: FilterMode::Client,
            input_stage: InputStage::SelectPacket,
            editing: false,
            preedit_text: String::new(),
            menus,
            displayed_menu: 0,
            edit_index: None,
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

    pub fn displayed_menu_mut(&mut self) -> &mut StatefulList<String>{
        let index = self.displayed_menu;
        match index {
            0 => &mut self.menus[0],
            1 => &mut self.menus[1],
            2 => &mut self.menus[2],
            _ => unimplemented!()
        }
    }
}

#[derive(PartialEq)]
pub enum InputStage {
    SelectPacket,
    SelectCommand,
    CommandModification,
    SendCommand,
}

impl InputStage {
    pub fn next(&mut self) {
        *self = match self {
            InputStage::SelectPacket => InputStage::SelectCommand,
            InputStage::SelectCommand=> InputStage::CommandModification,
            InputStage::CommandModification => InputStage::SendCommand,
            InputStage::SendCommand => InputStage::SelectPacket,
        };
    }

    pub fn prev(&mut self) {
        *self = match self {
            InputStage::SelectPacket => InputStage::SelectPacket,
            InputStage::SelectCommand => InputStage::SelectPacket,
            InputStage::CommandModification => InputStage::SelectCommand,
            InputStage::SendCommand => InputStage::CommandModification,
        };
    }
}
