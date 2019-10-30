use std::collections::HashMap;

use anyhow::Error;
use chrono::Utc;
use path_abs::PathFile;
use termion::event::Key;
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Text, Widget},
};

use crate::{entry, utility};
use crate::entry::GooseberryEntryFormat;

pub struct GooseberryTabs {
    task_tab: TaskTab,
}

pub trait GooseberryTab {
    /// Returns the TUI layout for the Tab
    fn get_layout(&self) -> Layout;
    /// Renders the text inside the Tab
    fn render(&self, chunks: &[Rect], frame: &mut utility::interactive::TuiFrame);
    /// Deals with key-press events inside the Tab
    fn keypress(&mut self, key: Key) -> Result<bool, Error>;
}

impl GooseberryTab for TaskTab {
    fn get_layout(&self) -> Layout {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(5)
            .constraints(self.get_constraints().as_ref())
    }

    fn render(&self, chunks: &[Rect], frame: &mut utility::interactive::TuiFrame) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title_style(Style::default().modifier(Modifier::BOLD));
        Paragraph::new(self.get_texts().iter())
            .block(block.clone().title("Tasks"))
            .alignment(Alignment::Left)
            .render(frame, chunks[0]);
        if self.is_writing() {
            self.input_boxes.render(&chunks[1..], frame);
        }
    }

    fn keypress(&mut self, key: Key) -> Result<bool, Error> {
        if self.input_boxes.is_writing {
            if let Some(task_entry) = self.input_boxes.keypress(key) {
                self.add_entry(task_entry)?;
            }
        } else if let Key::Char(c) = key {
            match c {
                'q' => return Ok(true),
                'n' => self.input_boxes.start_writing(),
                _ => (),
            }
        }

        Ok(false)
    }
}

pub struct TaskTab {
    fold: bool,
    entries: HashMap<u64, entry::TaskEntry>,
    visible_ids: Vec<u64>,
    input_boxes: utility::interactive::InputBoxes,
    next_id: u64,
}

impl TaskTab {
    pub fn is_writing(&self) -> bool {
        self.input_boxes.is_writing
    }

    pub fn new(entries: HashMap<u64, entry::TaskEntry>, next_id: u64) -> Self {
        let visible_ids = entries.keys().cloned().collect();
        TaskTab {
            entries,
            fold: false,
            visible_ids,
            input_boxes: utility::interactive::InputBoxes::new(vec![
                utility::interactive::InputBox::new(String::from("Task"), false, 10),
                utility::interactive::InputBox::new(String::from("Description"), true, 70),
            ]),
            next_id,
        }
    }

    fn get_constraints(&self) -> Vec<Constraint> {
        if self.input_boxes.is_writing {
            let mut contraints = vec![Constraint::Percentage(20)];
            contraints.extend_from_slice(&self.input_boxes.get_constraints());
            contraints
        } else {
            vec![Constraint::Percentage(100)]
        }
    }

    fn get_texts(&self) -> Vec<Text> {
        self.visible_ids
            .iter()
            .flat_map(|i| {
                if self.fold {
                    self.entries[&i].to_tui_short().unwrap()
                } else {
                    self.entries[&i].to_tui_long().unwrap()
                }
            })
            .collect()
    }

    fn add_entry(&mut self, boxes: Vec<utility::interactive::InputBox>) -> Result<(), Error> {
        let (task, description) = (boxes[0].get_content(), boxes[1].get_content());
        let task_entry = entry::TaskEntry {
            id: self.next_id,
            task,
            description,
            datetime: Utc::now(),
            done: false,
            tags: Vec::new(),
        };
        task_entry.to_file(PathFile::create(format!(
            "/Users/janani/PycharmProjects/rust-projects/gooseberry-tm/test_entries/{}.md",
            self.next_id
        ))?)?;
        self.entries.insert(self.next_id, task_entry);
        self.visible_ids.push(self.next_id);
        self.next_id += 1;
        Ok(())
    }
}
