use crate::{app::App, statefullist::StatefulList};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw_menu_list<B: Backend, L: std::fmt::Display>(
    f: &mut Frame<B>,
    list: &mut StatefulList<L>,
    mode: &str,
    screen_chunk: Rect,
) {
    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = list
        .items
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let lines = vec![Spans::from(format!("{}. {}", i, c))];
            ListItem::new(lines).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let cmd_list_title = format!("{} Menu", mode);
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(cmd_list_title))
        .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    // We can now render the item list
    f.render_stateful_widget(items, screen_chunk, &mut list.state);
}
