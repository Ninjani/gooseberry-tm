#![allow(dead_code)]
#[macro_use]
extern crate lazy_static;

use std::{
    collections::HashMap,
    io::{self, Write},
};

use anyhow::Error;
use path_abs::PathFile;
use termion::{input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};

use crate::entry::GooseberryEntryTrait;
use crate::tabs::GooseberryTab;

pub mod entry;
pub mod errors;
pub mod tabs;
pub mod utility;

fn gooseberry() -> Result<(), Error> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = utility::interactive::Events::default();
    let mut entries = HashMap::new();
    let (_, header, lines) = entry::get_type_header_lines(&PathFile::new(
        "/Users/janani/PycharmProjects/rust-projects/gooseberry-tm/test_entries/1.md",
    )?)?;
    entries.insert(1u64, entry::TaskEntry::from_header_lines(header, lines)?);
    let mut app = tabs::TaskTab::new(entries, 2);
    loop {
        terminal.draw(|mut f| {
            let size = f.size();
            let chunks = app.get_layout().split(size);
            app.render(&chunks, &mut f);
        })?;
        if app.is_writing() {
            io::stdout().flush().ok();
        }

        if let utility::interactive::Event::Input(key) = events.next()? {
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
