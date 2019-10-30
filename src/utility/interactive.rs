use std::{
    io::{self, Stdout},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::Error;
use dialoguer::{Editor, Input, theme as dialoguer_theme};
use termion::{
    event::Key, input::MouseTerminal, input::TermRead, raw::RawTerminal, screen::AlternateScreen,
};
use tui::{
    backend::TermionBackend,
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Text, Widget},
};

use crate::errors::Sorry;
use crate::utility;

pub type TuiFrame<'a> =
Frame<'a, TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>>;

#[derive(Debug, Clone)]
pub struct InputBoxes {
    pub is_writing: bool,
    boxes: Vec<InputBox>,
    index: usize,
}

#[derive(Debug, Clone)]
pub struct InputBox {
    title: String,
    is_writing: bool,
    content: String,
    markdown: bool,
    constraint: Constraint,
}

impl InputBox {
    pub fn new(title: String, markdown: bool, percent: u16) -> Self {
        Self {
            title,
            is_writing: false,
            content: String::new(),
            markdown,
            constraint: Constraint::Percentage(percent),
        }
    }

    pub fn get_content(&self) -> String {
        self.content.clone()
    }

    pub fn render(&self, chunk: Rect, frame: &mut TuiFrame) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title_style(Style::default());
        Paragraph::new(self.get_text().iter())
            .block(block.title(&self.title))
            .render(frame, chunk);
    }

    fn get_text(&self) -> Vec<Text> {
        let mut current = if self.markdown {
            utility::formatting::markdown_to_styled_texts(&self.content)
        } else {
            vec![Text::raw(&self.content)]
        };
        if self.is_writing {
            current.push(utility::formatting::cursor());
        }
        current
    }
}

impl InputBoxes {
    pub fn new(boxes: Vec<InputBox>) -> Self {
        Self {
            boxes,
            index: 0,
            is_writing: false,
        }
    }

    pub fn render(&self, chunks: &[Rect], frame: &mut TuiFrame) {
        for (i, chunk) in chunks.iter().enumerate() {
            self.boxes[i].render(*chunk, frame);
        }
    }

    pub fn start_writing(&mut self) {
        self.is_writing = true;
        self.index = 0;
        for i in 0..self.len() {
            self.boxes[i].is_writing = false;
        }
        self.boxes[self.index].is_writing = true;
    }

    pub fn get_constraints(&self) -> Vec<Constraint> {
        self.boxes.iter().map(|b| b.constraint).collect()
    }

    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }

    fn save(&mut self) -> Vec<InputBox> {
        self.is_writing = false;
        self.index = 0;
        let boxes = self.boxes.clone();
        for i in 0..self.len() {
            self.boxes[i].is_writing = false;
            self.boxes[i].content = String::new();
        }
        boxes
    }

    fn increment_box(&mut self) {
        self.boxes[self.index].is_writing = false;
        self.index += 1;
        if self.index >= self.len() {
            self.index = 0;
        }
        self.boxes[self.index].is_writing = true;
    }

    fn decrement_box(&mut self) {
        self.boxes[self.index].is_writing = false;
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.len() - 1;
        }
        self.boxes[self.index].is_writing = true;
    }

    pub fn keypress(&mut self, key: Key) -> Option<Vec<InputBox>> {
        match key {
            Key::Ctrl(c) => match c {
                's' => return Some(self.save()),
                'n' => self.increment_box(),
                'b' => self.decrement_box(),
                _ => (),
            },
            Key::Char(c) => {
                if !self.boxes[self.index].markdown && c == '\n' {
                    self.increment_box()
                } else {
                    self.boxes[self.index].content.push(c)
                }
            }
            Key::Backspace => {
                self.boxes[self.index].content.pop();
            }
            Key::Esc => self.is_writing = false,
            _ => (),
        }
        None
    }
}

/// Gets input from external editor, optionally displays default text in editor
pub fn external_editor_input(default: Option<&str>) -> Result<String, Error> {
    match Editor::new().edit(default.unwrap_or(""))? {
        Some(input) => Ok(input),
        None => Err(Sorry::EditorError.into()),
    }
}

/// Takes user input from terminal, optionally has a default and optionally displays it.
pub fn user_input(
    message: &str,
    default: Option<&str>,
    show_default: bool,
) -> Result<String, Error> {
    match default {
        Some(default) => Ok(
            Input::with_theme(&dialoguer_theme::ColorfulTheme::default())
                .with_prompt(message)
                .default(default.to_owned())
                .show_default(show_default)
                .interact()?
                .trim()
                .to_owned(),
        ),
        None => Ok(
            Input::<String>::with_theme(&dialoguer_theme::ColorfulTheme::default())
                .with_prompt(message)
                .interact()?
                .trim()
                .to_owned(),
        ),
    }
}

pub enum Event<I> {
    Input(I),
    Tick,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Default for Events {
    fn default() -> Self {
        Events::with_config(Config::default())
    }
}

impl Events {
    pub fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    if let Ok(key) = evt {
                        if tx.send(Event::Input(key)).is_err() {
                            return;
                        }
                        if key == config.exit_key {
                            return;
                        }
                    }
                }
            })
        };
        let tick_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let tx = tx.clone();
                loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(config.tick_rate);
                }
            })
        };
        Events {
            rx,
            input_handle,
            tick_handle,
        }
    }

    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}

pub struct TabsState<'a> {
    pub titles: Vec<&'a str>,
    pub index: usize,
}

impl<'a> TabsState<'a> {
    pub fn new(titles: Vec<&'a str>) -> TabsState {
        TabsState { titles, index: 0 }
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}
