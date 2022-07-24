pub(crate) mod app;
mod input;
pub(crate) mod statefullist;
mod ui;

use app::{App, InputStage};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use netwaystev2::filter::FilterMode;
use statefullist::StatefulList;
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let packet_selections = create_packet_selection_lists(FilterMode::Client);

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new(FilterMode::Client, packet_selections);
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
                    InputStage::CommandModification => {}
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
}

fn create_packet_selection_lists(mode: FilterMode) -> Vec<StatefulList<String>> {
    match mode {
        FilterMode::Client => {
            let client_packets = StatefulList::with_items(vec!["RequestAction".to_owned(), "ResponseCode".to_owned()]);

            let ra_list = StatefulList::with_items(vec!["RA_one".to_owned(), "RA_two".to_owned()]);

            let rc_list = StatefulList::with_items(vec!["RC_one".to_owned(), "RC_two".to_owned()]);

            vec![client_packets, ra_list, rc_list]
        }
        FilterMode::Server => {
            // TODO
            vec![]
        }
    }
}
