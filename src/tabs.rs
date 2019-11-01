use std::collections::HashMap;

use anyhow::Error;
use crossterm::KeyEvent;
use glob::glob;
use path_abs::{PathDir, PathFile};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs, Text, Widget},
};

use crate::{entry, utility};
use crate::entry::GooseberryEntryTrait;

pub struct GooseberryTabs {
    pub tabs: Vec<GooseberryTab>,
    pub index: usize,
}

impl GooseberryTabs {
    pub fn from_folder(folder: &PathDir) -> Result<Self, Error> {
        Ok(Self {
            tabs: vec![
                GooseberryTab::from_folder(entry::GooseberryEntryType::Task, folder)?,
                GooseberryTab::from_folder(entry::GooseberryEntryType::Journal, folder)?,
                GooseberryTab::from_folder(entry::GooseberryEntryType::Research, folder)?,
                GooseberryTab::from_folder(entry::GooseberryEntryType::Event, folder)?,
            ],
            index: 0,
        })
    }

    pub fn render(&self, frame: &mut utility::interactive::TuiFrame) {
        let titles = self
            .tabs
            .iter()
            .map(|t| t.title.clone())
            .collect::<Vec<_>>();
        let mut tabs = Tabs::default()
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .titles(&titles)
            .select(self.index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::Yellow));
        self.tabs[self.index].render(frame, &mut tabs);
    }

    pub fn is_writing(&self) -> bool {
        self.tabs[self.index].is_writing()
    }

    pub fn keypress(&mut self, key: KeyEvent) -> Result<bool, Error> {
        if !self.is_writing() {
            match key {
                KeyEvent::Right => self.next(),
                KeyEvent::Left => self.previous(),
                _key => return self.tabs[self.index].keypress(_key),
            }
        } else {
            return self.tabs[self.index].keypress(key);
        }
        Ok(false)
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.tabs.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.tabs.len() - 1;
        }
    }
}

pub struct GooseberryTab {
    title: String,
    entry_type: entry::GooseberryEntryType,
    fold: bool,
    entries: HashMap<u64, entry::GooseberryEntry>,
    visible_ids: Vec<u64>,
    input_boxes: utility::interactive::InputBoxes,
    next_id: u64,
    folder: PathDir,
}

impl GooseberryTab {
    pub fn get_layout(&self) -> Layout {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(5)
            .constraints(self.get_constraints().as_ref())
    }

    pub fn render(&self, frame: &mut utility::interactive::TuiFrame, tabs: &mut Tabs<String>) {
        let size = frame.size();
        let chunks = self.get_layout().split(size);
        tabs.render(frame, chunks[0]);
        let block = Block::default()
            .borders(Borders::ALL)
            .title_style(Style::default().modifier(Modifier::BOLD));
        Paragraph::new(self.get_texts().iter())
            .block(block.clone().title(&self.title))
            .alignment(Alignment::Left)
            .render(frame, chunks[1]);
        if self.is_writing() {
            self.input_boxes.render(&chunks[2..], frame);
        }
    }

    pub fn keypress(&mut self, key: KeyEvent) -> Result<bool, Error> {
        if self.input_boxes.is_writing {
            if let Some(task_entry_boxes) = self.input_boxes.keypress(key) {
                self.add_entry(task_entry_boxes)?;
            }
        } else if let KeyEvent::Char(c) = key {
            match c {
                'q' => return Ok(true),
                'n' => self.input_boxes.start_writing(),
                '\t' => self.toggle_fold(),
                _ => (),
            }
        }

        Ok(false)
    }

    pub fn toggle_fold(&mut self) {
        self.fold = !self.fold;
    }

    pub fn is_writing(&self) -> bool {
        self.input_boxes.is_writing
    }

    pub fn from_folder(
        entry_type: entry::GooseberryEntryType,
        folder: &PathDir,
    ) -> Result<Self, Error> {
        let mut entries = HashMap::new();
        let mut visible_ids = Vec::new();
        for file in glob(&format!(
            "{}/{}_*.md",
            folder.as_path().display(),
            entry_type
        ))? {
            let g_entry = entry::GooseberryEntry::from_file(&PathFile::new(file?)?)?;

            visible_ids.push(g_entry.id());
            entries.insert(g_entry.id(), g_entry);
        }
        let next_id = *visible_ids.iter().max().unwrap_or(&0) + 1;
        Ok(GooseberryTab {
            title: format!("{}", entry_type),
            entries,
            fold: false,
            visible_ids,
            input_boxes: entry_type.get_input_boxes(),
            next_id,
            folder: folder.to_owned(),
            entry_type,
        })
    }

    fn get_constraints(&self) -> Vec<Constraint> {
        self.input_boxes.get_constraints()
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
        let new_entry =
            entry::GooseberryEntry::from_input_boxes(self.next_id, self.entry_type, boxes)?;
        new_entry.to_file(PathFile::create(
            self.entry_type.get_file(&self.folder, self.next_id)?,
        )?)?;
        self.entries.insert(self.next_id, new_entry);
        self.visible_ids.push(self.next_id);
        self.next_id += 1;
        Ok(())
    }
}
