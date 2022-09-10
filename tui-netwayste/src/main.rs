pub(crate) mod app;
pub(crate) mod fieldeditlist;
mod input;
mod nw;
mod nw_protocol;
pub(crate) mod statefullist;
mod ui;

use app::{App, InputStage};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fieldeditlist::FieldEditList;
use netwaystev2::filter::FilterMode;
use nw::{create_packet_selection_lists, create_request_action_data};
use statefullist::StatefulList;
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
    vec,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    Frame, Terminal,
};

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let packet_selections = create_packet_selection_lists(FilterMode::Client);
    let request_actions = create_request_action_data();

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new(FilterMode::Client, packet_selections, request_actions);
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
            draw_app(f, &mut app);
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
                    InputStage::SelectPacket => input::handle_list_navigation(key.code, &mut app),
                    InputStage::SelectCommand => input::handle_list_navigation(key.code, &mut app),
                    InputStage::CommandModification => {
                        build_field_edit_list(&mut app);
                        input::handle_command_modification(key.code, &mut app);
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

pub fn draw_app<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create two chunks with 20/80 ratio horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(f.size());

    ui::draw_menu_list(f, app.displayed_menu_mut(), "Client", chunks[0]);
    ui::draw_event_log(f, app, chunks[1]);
    ui::draw_edit_ui(f, app);
}

use crate::nw::get_mimic_meta_from;
use std::fs::File;
use std::io::prelude::*;
pub fn build_field_edit_list(app: &mut App) {
    if app.edit_list_state.is_some() {
        return;
    }

    // TODO: Select either ra_data or rc_data
    let command_index = app.displayed_menu_mut().get_index();
    let mimic_metadata = get_mimic_meta_from(&app.ra_data[command_index]);

    // Iterate through all elements in the `items` app and append some debug text to it.
    if let Some(metadata) = mimic_metadata {
        let mut fields = vec![];
        // FIXME: MetadataField needs to be bound to Iterator
        for field in &metadata.fields {
            let key = format!("{} ({})\n", field.name, field.type_,);
            fields.push(key);
        }
        app.edit_list_state = Some(FieldEditList::with_fields(fields));
    }
}
