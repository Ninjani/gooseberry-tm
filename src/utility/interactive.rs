use std::{sync::mpsc, thread, time::Duration};

use crossterm::{input, InputEvent, KeyEvent};
use tui::{
    backend::CrosstermBackend,
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Text, Widget},
};

use crate::gooseberry_app::{HELP_BOX_PERCENT, TAB_BOX_PERCENT};
use crate::utility;

pub type TuiFrame<'a> = Frame<'a, CrosstermBackend>;

#[derive(Debug, Clone)]
pub struct InputBoxes {
    /// List of text input boxes
    /// This should really be a list of things that you can take input from
    /// TODO: Replace with an enum to include a choice box for task states and other multiple choice stuff
    boxes: Vec<InputBox>,
    /// Index to the active box
    index: usize,
}

#[derive(Debug, Clone)]
pub struct InputBox {
    /// Written on top of the box
    title: String,
    /// true if it's the active box being written to
    is_writing: bool,
    /// growing content of the box
    content: String,
    /// if true, renders markdown, else plain text
    /// TODO: Probably make this more flexible, e.g. code?
    markdown: bool,
    /// How much of the terminal should it take up
    /// This is a bit weird right now, you have to make sure not to cover up the help and tab bars
    percent: u16,
    /// scroll index
    scroll: u16,
}

impl InputBox {
    /// Makes a new empty box
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

    /// Retrieves the content inside the box
    pub fn get_content(&self) -> String {
        self.content.clone()
    }

    /// Renders the box as a bounded paragraph with a title, wrapped text, and scroll
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

    /// Styles text according to whether self.markdown is true or not
    /// TODO: Again, flexibility
    /// Also, adds a fake cursor to the end if it's the active box
    /// Doesn't handle moving around with arrow keys, pretty clunky that way, you have to backspace
    /// TODO: Switch to ropey and keep an index to deal with this^?
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
    /// Makes a new struct with active box as the first one
    pub fn new(boxes: Vec<InputBox>) -> Self {
        let mut input_boxes = Self { boxes, index: 0 };
        input_boxes.start_writing();
        input_boxes
    }

    /// Renders all the boxes
    pub fn render(&self, chunks: &[Rect], frame: &mut TuiFrame) {
        for (i, chunk) in chunks.iter().enumerate() {
            self.boxes[i].render(*chunk, frame);
        }
    }

    /// Sets the first box to active, and turns the others off (for writing, not rendering)
    /// This should make it so that only one box has the fake cursor
    /// But `\t` seems to break this for some reason
    /// I think the 0.12 release of `crossterm` should fix this as they have Tab as a separate KeyEvent
    pub fn start_writing(&mut self) {
        self.index = 0;
        for i in 0..self.len() {
            self.boxes[i].is_writing = false;
        }
        self.boxes[self.index].is_writing = true;
    }

    /// Turns off all the boxes for writing
    pub fn stop_writing(&mut self) {
        self.index = 0;
        for i in 0..self.len() {
            self.boxes[i].is_writing = false;
        }
    }

    /// Replaces the content in a specified box
    /// TODO: BOUNDS CHECK!!!
    pub fn replace_content(&mut self, index: usize, content: &str) {
        self.boxes[index].content = content.to_owned();
    }

    /// Makes layout constraints based on the percentages of each box
    /// Starts with the tab bar
    /// then the boxes
    /// then the help bar
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

    /// I don't know why I do this
    pub fn len(&self) -> usize {
        self.boxes.len()
    }

    /// Or this
    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }

    /// Return the boxes and clear them out. Stop writing on save
    fn save(&mut self) -> Vec<InputBox> {
        let boxes = self.boxes.clone();
        for i in 0..self.len() {
            self.boxes[i].content = String::new();
        }
        self.stop_writing();
        boxes
    }

    /// Go to the next box (wraps around)
    fn increment_box(&mut self) {
        self.boxes[self.index].is_writing = false;
        self.index = (self.index + 1) % self.len();
        self.boxes[self.index].is_writing = true;
    }

    /// Go to the previous box (wraps around)
    fn decrement_box(&mut self) {
        self.boxes[self.index].is_writing = false;
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.len() - 1;
        }
        self.boxes[self.index].is_writing = true;
    }

    /// Handle keyboard input events
    /// Ctrl-s: saves the entry being written
    /// Ctrl-n: next (next box)
    /// Ctrl-b: back (previous box) TODO: Use next, previous or forward, backward ugh
    /// `\n`: if markdown=false then go to the next box, otherwise it's a normal `\n`
    /// Backspace: deletes a character
    /// ^ (Up arrow): scrolls up
    /// v (Down arrow): scrolls down
    /// Esc: pauses writing mode to go back to scrolling mode.
    ///     Pressing n again resumes writing mode at the same state
    /// Returns (a potential new entry to save, an indicator of whether to stop writing mode)
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

/// Copied from `tui`/examples/util.rs
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
    /// No quit key, that's handled elsewhere
    fn default() -> Self {
        Events::new(Duration::from_millis(250))
    }
}

/// There's a ton of clones in here, probably necessary?
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
