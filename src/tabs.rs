use std::collections::HashMap;

use anyhow::Error;
use crossterm::KeyEvent;
use glob::glob;
use path_abs::{PathDir, PathFile};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs, Text, Widget},
};

use crate::{entry, utility};
use crate::entry::GooseberryEntryTrait;
use crate::errors::Sorry;

const HELP_TEXT: &str =
    "< > : change tabs, ^ v : scroll, n : new entry/resume editing, e <id>[Enter] : edit entry, \\t : toggle fold, q : quit";
const WRITING_HELP_TEXT: &str =
    "Ctrl-n : next box, Ctrl-b : previous box, Ctrl-s : save, Esc : pause writing";

pub(crate) const TAB_BOX_PERCENT: u16 = 7;
pub(crate) const HELP_BOX_PERCENT: u16 = 13;

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
            .block(Block::default().borders(Borders::ALL))
            .titles(&titles)
            .select(self.index)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(Style::default().fg(Color::Yellow));
        self.tabs[self.index].render(frame, &mut tabs);
    }

    pub fn is_writing(&self) -> bool {
        self.tabs[self.index].is_writing
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
    is_writing: bool,
    input_boxes: utility::interactive::InputBoxes,
    next_id: u64,
    folder: PathDir,
    scroll: u16,
    picking_char: Option<char>,
    picking_entry: bool,
    selected_entry: u64,
    editing_id: Option<u64>,
}

impl GooseberryTab {
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
            is_writing: false,
            input_boxes: entry_type.get_input_boxes(),
            next_id,
            folder: folder.to_owned(),
            entry_type,
            scroll: 0,
            selected_entry: 0,
            editing_id: None,
            picking_entry: false,
            picking_char: None,
        })
    }

    pub fn get_layout(&self) -> Layout {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(self.get_constraints().as_ref())
    }

    fn render_help_box(&self, frame: &mut utility::interactive::TuiFrame, chunk: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title_style(Style::default().modifier(Modifier::BOLD));
        if self.is_writing {
            Paragraph::new(vec![Text::Raw(WRITING_HELP_TEXT.into())].iter())
                .block(block)
                .alignment(Alignment::Center)
                .wrap(true)
                .render(frame, chunk)
        } else {
            Paragraph::new(vec![Text::Raw(HELP_TEXT.into())].iter())
                .block(block.clone())
                .alignment(Alignment::Center)
                .wrap(true)
                .render(frame, chunk)
        }
    }

    pub fn render(&self, frame: &mut utility::interactive::TuiFrame, tabs: &mut Tabs<String>) {
        let size = frame.size();
        let chunks = self.get_layout().split(size);
        tabs.render(frame, chunks[0]);
        let block = Block::default()
            .borders(Borders::ALL)
            .title_style(Style::default().modifier(Modifier::BOLD));
        Paragraph::new(self.get_texts().iter())
            .block(block.clone())
            .alignment(Alignment::Left)
            .scroll(self.scroll)
            .wrap(true)
            .render(frame, chunks[1]);
        if self.is_writing {
            self.input_boxes.render(&chunks[2..chunks.len() - 1], frame);
        }
        self.render_help_box(frame, chunks[chunks.len() - 1]);
    }

    fn toggle_task_entry(&mut self) -> Result<(), Error> {
        if self.entry_type == entry::GooseberryEntryType::Task {
            let t_entry =
                self.entries
                    .get_mut(&self.selected_entry)
                    .ok_or(Sorry::WrongEntryID {
                        entry_type: self.entry_type,
                        entry_id: self.selected_entry,
                    })?;
            if let entry::GooseberryEntry::Task(ref mut t) = t_entry {
                t.toggle();
            }
            self.save_entry(self.selected_entry)?;
        }
        Ok(())
    }

    pub fn keypress(&mut self, key: KeyEvent) -> Result<bool, Error> {
        if self.is_writing {
            let (new_entry, stop_writing) = self.input_boxes.keypress(key);
            if let Some(new_entry) = new_entry {
                if let Some(id) = self.editing_id {
                    self.add_entry(new_entry, id)?;
                } else {
                    self.add_entry(new_entry, self.next_id)?;
                    self.next_id += 1;
                }
            }
            if stop_writing {
                self.is_writing = false;
            }
        } else {
            match key {
                KeyEvent::Char(c) => match c {
                    '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' => {
                        if self.picking_entry {
                            if self.selected_entry > 0 {
                                self.selected_entry =
                                    format!("{}{}", self.selected_entry, c).parse()?;
                            } else {
                                self.selected_entry = c.to_string().parse()?;
                            }
                        }
                    }
                    '\n' => {
                        if let Some(c) = self.picking_char {
                            match c {
                                't' => self.toggle_task_entry()?,
                                'e' => self.edit_entry()?,
                                _ => (),
                            }
                        }
                        self.picking_entry = false;
                        self.selected_entry = 0;
                        self.picking_char = None;
                    }
                    'q' => return Ok(true),
                    'n' => {
                        self.input_boxes.start_writing();
                        self.is_writing = true;
                    }
                    '\t' => self.toggle_fold(),
                    't' | 'e' => {
                        self.picking_char = Some(c);
                        self.picking_entry = true;
                        self.selected_entry = 0;
                    }
                    _ => (),
                },
                KeyEvent::Down => self.scroll += 1,
                KeyEvent::Up => {
                    if self.scroll > 0 {
                        self.scroll -= 1;
                    }
                }
                _ => (),
            }
        }
        Ok(false)
    }

    pub fn toggle_fold(&mut self) {
        self.fold = !self.fold;
    }

    fn get_constraints(&self) -> Vec<Constraint> {
        if self.is_writing {
            self.input_boxes.get_constraints()
        } else {
            vec![
                Constraint::Percentage(TAB_BOX_PERCENT),
                Constraint::Percentage(100 - TAB_BOX_PERCENT - HELP_BOX_PERCENT),
                Constraint::Percentage(HELP_BOX_PERCENT),
            ]
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

    fn edit_entry(&mut self) -> Result<(), Error> {
        self.input_boxes = self
            .entries
            .get(&self.selected_entry)
            .ok_or(Sorry::WrongEntryID {
                entry_type: self.entry_type,
                entry_id: self.selected_entry,
            })?
            .to_input_boxes();
        self.is_writing = true;
        self.editing_id = Some(self.selected_entry);
        Ok(())
    }

    fn save_entry(&self, id: u64) -> Result<(), Error> {
        self.entries
            .get(&id)
            .ok_or(Sorry::WrongEntryID {
                entry_type: self.entry_type,
                entry_id: id,
            })?
            .to_file(PathFile::create(
                self.entry_type.get_file(&self.folder, id)?,
            )?)?;
        Ok(())
    }

    fn add_entry(&mut self, boxes: Vec<utility::interactive::InputBox>, id: u64) -> Result<(), Error> {
        let new_entry =
            entry::GooseberryEntry::from_input_boxes(id, self.entry_type, boxes)?;
        if self.entries.insert(id, new_entry).is_none() {
            self.visible_ids.push(id);
        }
        self.save_entry(id)?;
        Ok(())
    }
}
