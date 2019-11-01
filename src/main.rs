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
pub mod tabs;
pub mod utility;

fn gooseberry() -> Result<(), Error> {
    // Terminal initialization
    let screen = AlternateScreen::to_alternate(true)?;
    let backend = CrosstermBackend::with_alternate_screen(screen)?;
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = utility::interactive::Events::default();

    let mut app = tabs::GooseberryTabs::from_folder(&PathDir::new(
        "/Users/janani/PycharmProjects/rust-projects/gooseberry-tm/test_entries",
    )?)?;
    terminal.clear()?;
    loop {
        terminal.draw(|mut f| app.render(&mut f))?;
        if app.is_writing() {
            io::stdout().flush().ok();
        }
        if let Ok(utility::interactive::Event::Input(key)) = events.next() {
            if app.keypress(key)? {
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
