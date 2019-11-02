#![allow(dead_code)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use std::io::{self, Write};

use anyhow::Error;
use crossterm::AlternateScreen;
use path_abs::PathDir;
use tui::{backend::CrosstermBackend, Terminal};

pub mod entry;
pub mod errors;
pub mod app;
pub mod utility;

fn main() -> Result<(), Error> {
    // Terminal initialization
    let screen = AlternateScreen::to_alternate(true)?;
    let backend = CrosstermBackend::with_alternate_screen(screen)?;
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Keep track of keyboard events
    let events = utility::interactive::Events::default();

    // App
    let mut gooseberry = app::GooseberryTabs::from_folder(&PathDir::new("test_entries")?)?;
    terminal.clear()?;

    // Main rendering loop
    loop {
        terminal.draw(|mut f| gooseberry.render(&mut f))?;

        // flush immediately so that you see each character as you type
        if gooseberry.is_writing() {
            io::stdout().flush().ok();
        }

        // Handle keyboard input
        if let Ok(utility::interactive::Event::Input(key)) = events.next() {
            let should_break = gooseberry.keypress(key)?;
            if should_break {
                break;
            }
        }
    }
    Ok(())
}
