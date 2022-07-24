use crate::app::App;

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw_event_log<B: Backend>(f: &mut Frame<B>, app: &mut App, screen_chunk: Rect) {
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
                Spans::from("-".repeat(screen_chunk.width as usize)),
                header,
                Spans::from(""),
                log,
            ])
        })
        .collect();

    let events_list = List::new(events)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .start_corner(Corner::BottomLeft);

    f.render_widget(events_list, screen_chunk);
}
