use std::{sync::mpsc, thread, time::Duration};

use anyhow::Error;
use crossterm::{input, InputEvent, KeyEvent};
use dialoguer::{Editor, Input, theme as dialoguer_theme};
use tui::{
    backend::CrosstermBackend,
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Text, Widget},
};

use crate::errors::Sorry;
use crate::utility;

pub type TuiFrame<'a> = Frame<'a, CrosstermBackend>;
//    Frame<'a, TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>>;

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
    percent: u16,
}

impl InputBox {
    pub fn new(title: String, markdown: bool, percent: u16) -> Self {
        Self {
            title,
            is_writing: false,
            content: String::new(),
            markdown,
            percent,
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

    pub fn stop_writing(&mut self) {
        self.is_writing = false;
        self.index = 0;
        for i in 0..self.len() {
            self.boxes[i].is_writing = false;
        }
    }

    pub fn get_constraints(&self) -> Vec<Constraint> {
        if self.is_writing {
            let sum = self.boxes.iter().map(|b| b.percent).sum::<u16>();
            let first_box_percent = 10;
            let second_box_percent = 100 - 10 - sum;
            let mut constraints = vec![
                Constraint::Percentage(first_box_percent),
                Constraint::Percentage(second_box_percent),
            ];
            constraints.extend(self.boxes.iter().map(|b| Constraint::Percentage(b.percent)));
            constraints
        } else {
            vec![Constraint::Percentage(10), Constraint::Percentage(90)]
        }
    }

    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }

    fn save(&mut self) -> Vec<InputBox> {
        let boxes = self.boxes.clone();
        for i in 0..self.len() {
            self.boxes[i].content = String::new();
        }
        self.stop_writing();
        boxes
    }

    fn increment_box(&mut self) {
        self.boxes[self.index].is_writing = false;
        self.index = (self.index + 1) % self.len();
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

    pub fn keypress(&mut self, key: KeyEvent) -> Option<Vec<InputBox>> {
        match key {
            KeyEvent::Ctrl(c) => match c {
                's' => return Some(self.save()),
                'n' => self.increment_box(),
                'b' => self.decrement_box(),
                _ => (),
            },
            KeyEvent::Char(c) => {
                if !self.boxes[self.index].markdown && c == '\n' {
                    self.increment_box()
                } else {
                    self.boxes[self.index].content.push(c)
                }
            }
            KeyEvent::Backspace => {
                self.boxes[self.index].content.pop();
            }
            KeyEvent::Esc => self.stop_writing(),
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
    rx: mpsc::Receiver<Event<KeyEvent>>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>,
}
//
//#[derive(Debug, Clone)]
//pub struct Config {
//    pub exit_key: KeyEvent,
//    pub tick_rate: Duration,
//}
//
//impl Default for Config {
//    fn default() -> Config {
//        Config {
//            exit_key: KeyEvent::Char('q'),
//            tick_rate: Duration::from_millis(250),
//        }
//    }
//}

impl Default for Events {
    fn default() -> Self {
        Events::new(Duration::from_millis(250), 'q')
    }
}

impl Events {
    pub fn new(tick_rate: Duration, exit_char: char) -> Events {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let input = input();
                let reader = input.read_sync();
                for evt in reader {
                    if let InputEvent::Keyboard(key) = evt {
                        if tx.send(Event::Input(key.clone())).is_err() {
                            return;
                        }
                        if key == KeyEvent::Char(exit_char) {
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
                    thread::sleep(tick_rate);
                }
            })
        };
        Events {
            rx,
            input_handle,
            tick_handle,
        }
    }

    pub fn next(&self) -> Result<Event<KeyEvent>, mpsc::RecvError> {
        self.rx.recv()
    }
}
