use std::{sync::mpsc, thread, time::Duration};

use crossterm::{input, InputEvent, KeyEvent};
use tui::{
    backend::CrosstermBackend,
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Text, Widget},
};

use crate::tabs::{HELP_BOX_PERCENT, TAB_BOX_PERCENT};
use crate::utility;

pub type TuiFrame<'a> = Frame<'a, CrosstermBackend>;

#[derive(Debug, Clone)]
pub struct InputBoxes {
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
    scroll: u16,
}

impl InputBox {
    pub fn new(title: String, markdown: bool, percent: u16) -> Self {
        Self {
            title,
            is_writing: false,
            content: String::new(),
            markdown,
            percent,
            scroll: 0,
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
            .scroll(self.scroll)
            .wrap(true)
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
        let mut input_boxes = Self { boxes, index: 0 };
        input_boxes.boxes[input_boxes.index].is_writing = true;
        input_boxes
    }

    pub fn render(&self, chunks: &[Rect], frame: &mut TuiFrame) {
        for (i, chunk) in chunks.iter().enumerate() {
            self.boxes[i].render(*chunk, frame);
        }
    }

    pub fn start_writing(&mut self) {
        self.index = 0;
        for i in 0..self.len() {
            self.boxes[i].is_writing = false;
        }
        self.boxes[self.index].is_writing = true;
    }

    pub fn stop_writing(&mut self) {
        self.index = 0;
        for i in 0..self.len() {
            self.boxes[i].is_writing = false;
        }
    }

    pub fn replace_content(&mut self, index: usize, content: &str) {
        self.boxes[index].content = content.to_owned();
    }

    pub fn get_constraints(&self) -> Vec<Constraint> {
        let sum = self.boxes.iter().map(|b| b.percent).sum::<u16>();
        let second_box_percent = 100 - TAB_BOX_PERCENT - sum - HELP_BOX_PERCENT;
        let mut constraints = vec![
            Constraint::Percentage(TAB_BOX_PERCENT),
            Constraint::Percentage(second_box_percent),
        ];
        constraints.extend(self.boxes.iter().map(|b| Constraint::Percentage(b.percent)));
        constraints.push(Constraint::Percentage(HELP_BOX_PERCENT));
        constraints
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

    pub fn keypress(&mut self, key: KeyEvent) -> (Option<Vec<InputBox>>, bool) {
        match key {
            KeyEvent::Ctrl(c) => match c {
                's' => return (Some(self.save()), true),
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
            KeyEvent::Up => {
                if self.boxes[self.index].scroll > 0 {
                    self.boxes[self.index].scroll -= 1;
                }
            }
            KeyEvent::Down => self.boxes[self.index].scroll += 1,
            KeyEvent::Esc => {
                self.stop_writing();
                return (None, true);
            }
            _ => (),
        }
        (None, false)
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

impl Default for Events {
    fn default() -> Self {
        Events::new(Duration::from_millis(250))
    }
}

impl Events {
    pub fn new(tick_rate: Duration) -> Events {
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
