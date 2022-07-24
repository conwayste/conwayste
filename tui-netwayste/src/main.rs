pub(crate) mod app;
pub(crate) mod statefullist;
mod ui;

use app::{App, InputStage, MenuItemEntry};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use strum::IntoEnumIterator;
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
                    InputStage::NavigateMenu => handle_command_keys(key.code, &mut app),
                    InputStage::NavigateEdit => {}
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

    ui::draw_menu_list(f, &mut app.displayed_menu, "Client", chunks[0]);
    ui::draw_event_log(f, app, chunks[1]);
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
