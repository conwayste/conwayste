pub(crate) mod statefullist;
use statefullist::StatefulList;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use netwaystev2::{
    filter::{FilterMode, FilterCmd},
    protocol::{BroadcastChatMessage, Packet, RequestAction, ResponseCode},
};
use std::{
    collections::HashMap,
    error::Error,
    io,
    time::{Duration, Instant},
    vec,
};
use strum::IntoEnumIterator;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq)]
enum InputStage {
    NavigateMenu,
    NavigateEdit,
    SendCommand,
}

impl InputStage {
    fn next(&mut self) {
        *self = match self {
            InputStage::NavigateMenu => InputStage::NavigateEdit,
            InputStage::NavigateEdit => InputStage::SendCommand,
            InputStage::SendCommand => InputStage::NavigateMenu,
        };
    }

    fn prev(&mut self) {
        *self = match self {
            InputStage::NavigateMenu => InputStage::NavigateMenu,
            InputStage::NavigateEdit => InputStage::NavigateMenu,
            InputStage::SendCommand => InputStage::NavigateEdit,
        };
    }
}

struct EditableCommand {
    fields: StatefulList<Field>,
}

impl EditableCommand {
    fn with_items(list_fields: Vec<Field>) -> Self {
        EditableCommand {
            fields: StatefulList::with_items(list_fields),
        }
    }
}

enum MenuItemEntry {
    MenuIndex(usize),
    EditDialog(EditableCommand),
}

/// This struct holds the current state of the app. In particular, it has the `items` field which is a wrapper
/// around `ListState`. Keeping track of the items state let us render the associated widget with its state
/// and have access to features such as natural scrolling.
///
/// Check the event handling at the bottom to see how to change the state on incoming events.
/// Check the drawing logic for items on how to specify the highlighting style for selected items.
struct App<'a> {
    mode:               FilterMode,
    input_stage:        InputStage,
    editing:            bool,   // Are we editing a field?
    preedit_text:       String, // Previous field value while editing it; restored on cancel
    displayed_menu:     StatefulList<String>,
    menu_display_index: usize, // Index into the following vec
    menus:              Vec<StatefulList<String>>,
    menu_item_map:      HashMap<String, MenuItemEntry>,
    displayed_editor:   StatefulList<Field>,
    events:             Vec<(&'a str, &'a str)>,
}

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

impl<'a> App<'a> {
    fn new() -> App<'a> {
        let mut menu_item_map = HashMap::new();
        menu_item_map.insert("RequestAction".to_owned(), MenuItemEntry::MenuIndex(1));
        menu_item_map.insert("ResponseCode".to_owned(), MenuItemEntry::MenuIndex(2));

        for ra in RequestAction::iter() {
            menu_item_map.insert(
                ra.to_string().clone(),
                MenuItemEntry::EditDialog(EditableCommand::with_items(vec![
                    Field::new("Field 1", "DefaultValue"),
                    Field::new("Field 2", "124"),
                    Field::new("Field 3", ""),
                ])),
            );
        }

        for rc in ResponseCode::iter() {
            menu_item_map.insert(
                rc.to_string().clone(),
                MenuItemEntry::EditDialog(EditableCommand::with_items(vec![
                    Field::new("Field 1", "DefaultValue"),
                    Field::new("Field 2", "124"),
                    Field::new("Field 3", ""),
                ])),
            );
        }

        let menus = vec![
            StatefulList::with_items(vec!["RequestAction".to_owned(), "ResponseCode".to_owned()]),
            StatefulList::with_items(RequestAction::iter().map(|ra| ra.to_string()).collect()),
            StatefulList::with_items(ResponseCode::iter().map(|rc| rc.to_string()).collect()),
        ];

        let menu_display_index = 0;
        let displayed_menu = menus[menu_display_index].clone();

        let displayed_editor = EditableCommand::with_items(vec![]).fields;

        App {
            mode: FilterMode::Client,
            input_stage: InputStage::NavigateMenu,
            editing: false,
            preedit_text: String::new(),
            displayed_menu,
            menu_display_index,
            menus,
            menu_item_map,
            displayed_editor,
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
    fn on_tick(&mut self) {
        let event = self.events.remove(0);
        self.events.push(event);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App, tick_rate: Duration) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| {
            menu_ui(f, &mut app);
            if app.input_stage == InputStage::NavigateEdit {
                // Drawing the menu first creates a nice pop-up effect
                editor_ui(f, &mut app);
            }
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if KeyCode::Char('q') == key.code {
                    return Ok(());
                }

                match app.input_stage {
                    InputStage::NavigateMenu => handle_command_keys(key.code, &mut app),
                    InputStage::NavigateEdit => {
                        if !app.editing {
                            handle_navigateedit_keys(key.code, &mut app)
                        } else {
                            handle_editcommand_keys(key.code, &mut app)
                        }
                    }
                    InputStage::SendCommand => (),
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn handle_command_keys(key: KeyCode, app: &mut App) {
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
            let mut opt_next_menu_idx = None;
            let mut opt_edit_cmd_ref = None;

            if let Some(index) = app.displayed_menu.state.selected() {
                let item_name = &app.displayed_menu.items[index];

                // Determine if the selected menu item leads to a sub-menu or an edit dialog. A submenu falls under a
                // request action or response code
                if let Some(entry) = app.menu_item_map.get(item_name) {
                    match entry {
                        MenuItemEntry::MenuIndex(next_menu_idx) => opt_next_menu_idx = Some(next_menu_idx),
                        MenuItemEntry::EditDialog(edit_cmd_ref) => opt_edit_cmd_ref = Some(edit_cmd_ref),
                    }
                }
            }

            if let Some(next_menu_index) = opt_next_menu_idx {
                // Save the state current menu and load the new menu
                let current_menu = std::mem::replace(&mut app.displayed_menu, app.menus[*next_menu_index].clone());
                app.menus[app.menu_display_index] = current_menu;
                app.menu_display_index = *next_menu_index;
            }

            if let Some(edit_cmd_ref) = opt_edit_cmd_ref {
                // Save the menu state; Load the edit state
                app.displayed_editor = edit_cmd_ref.fields.clone();

                app.input_stage.next();
            }
        }
        KeyCode::Esc => {
            if app.menu_display_index == 0 {
                return;
            }

            if app.input_stage == InputStage::NavigateMenu {
                // Save current menu state and load the parent menu
                app.menus[app.menu_display_index] = app.displayed_menu.clone();

                // HACK: We only have two levels of menus now. It isn't worth the time to build a proper menu tree
                app.menu_display_index = 0;
                app.displayed_menu = app.menus[app.menu_display_index].clone();
            } else {
                app.input_stage.prev();
            }
        }
        _ => {}
    }
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

            // Restore the original value
            app.displayed_editor.items[index].value = std::mem::take(&mut app.preedit_text);
        }
        _ => (),
    }
}

fn menu_ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(f.size());

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .displayed_menu
        .items
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let lines = vec![Spans::from(format!("{}. {}", i, c))];
            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let cmd_list_title = format!("{:?} Menu", app.mode);
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(cmd_list_title))
        .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    // We can now render the item list
    f.render_stateful_widget(items, chunks[0], &mut app.displayed_menu.state);

    // Let's do the same for the events.
    // The event list doesn't have any state and only displays the current state of the list.
    let events: Vec<ListItem> = app
        .events
        .iter()
        .rev()
        .map(|&(event, level)| {
            // Colorcode the level depending on its type
            let s = match level {
                "CRITICAL" => Style::default().fg(Color::Red),
                "ERROR" => Style::default().fg(Color::Magenta),
                "WARNING" => Style::default().fg(Color::Yellow),
                "INFO" => Style::default().fg(Color::Blue),
                _ => Style::default(),
            };
            // Add a example datetime and apply proper spacing between them
            let header = Spans::from(vec![
                Span::styled(format!("{:<9}", level), s),
                Span::raw(" "),
                Span::styled("2020-01-01 10:00:00", Style::default().add_modifier(Modifier::ITALIC)),
            ]);
            // The event gets its own line
            let log = Spans::from(vec![Span::raw(event)]);

            // Here several things happen:
            // 1. Add a `---` spacing line above the final list entry
            // 2. Add the Level + datetime
            // 3. Add a spacer line
            // 4. Add the actual event
            ListItem::new(vec![
                Spans::from("-".repeat(chunks[1].width as usize)),
                header,
                Spans::from(""),
                log,
            ])
        })
        .collect();
    let events_list = List::new(events)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .start_corner(Corner::BottomLeft);
    f.render_widget(events_list, chunks[1]);
}

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
