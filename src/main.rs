#![allow(dead_code)]
#[macro_use]
extern crate lazy_static;

use std::io::{self, Write};

use anyhow::Error;
use crossterm::AlternateScreen;
use path_abs::PathDir;
use tui::{backend::CrosstermBackend, Terminal};

pub mod entry;
pub mod errors;
pub mod gooseberry_app;
pub mod utility;

/// Shake the box
fn gooseberry() -> Result<(), Error> {
    /// Terminal initialization
    let screen = AlternateScreen::to_alternate(true)?;
    let backend = CrosstermBackend::with_alternate_screen(screen)?;
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    /// Keep track of keyboard events
    let events = utility::interactive::Events::default();
    let mut app = gooseberry_app::GooseberryTabs::from_folder(&PathDir::new(
        "/Users/janani/PycharmProjects/rust-projects/gooseberry-tm/test_entries",
    )?)?;
    terminal.clear()?;

    /// Main rendering loop
    loop {
        terminal.draw(|mut f| app.render(&mut f))?;

        /// flush immediately so that you see each character as you type
        if app.is_writing() {
            io::stdout().flush().ok();
        }

        /// Handle keyboard input
        if let Ok(utility::interactive::Event::Input(key)) = events.next() {
            let should_break = app.keypress(key)?;
            if should_break {
                break;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    gooseberry()?;
    Ok(())
}
