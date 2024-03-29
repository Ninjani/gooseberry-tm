use std::collections::HashMap;

use anyhow::Error;
use crossterm::cursor;
use crossterm::KeyEvent;
use crossterm::TerminalCursor;
use glob::glob;
use path_abs::{PathDir, PathFile};
use tui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs, Text, Widget},
};

use crate::{entry, utility, utility::config::CONFIG};
use crate::entry::GooseberryEntryTrait;
use crate::errors::Sorry;

//use tui::Terminal;
//use unicode_width::UnicodeWidthStr;
//use std::io::{self, Write};

/// Keyboard shortcuts in scrolling mode
const HELP_TEXT: &str =
    "< > : change tabs, ^ v : scroll\nn : new entry/resume editing, \
     e <id>[Enter] : edit entry, a <id>[Enter] : archive entry\n\\t : toggle fold\nt <id>[Enter] : toggle Task\nq : quit";

/// Keyboard shortcuts in writing mode
const WRITING_HELP_TEXT: &str =
    "Ctrl-n : next box, Ctrl-b : previous box\nCtrl-s : save, Esc : pause writing";

/// Percentage of the terminal to use for displaying the tab bar (on top)
pub(crate) const TAB_BOX_PERCENT: u16 = 7;
/// Percentage of the terminal to use for displaying the help text (at the bottom)
pub(crate) const HELP_BOX_PERCENT: u16 = 13;

/// Main application
pub struct GooseberryTabs {
    /// list of `GooseberryTab`s
    pub tabs: Vec<GooseberryTab>,
    /// index of active tab
    pub index: usize,
}

impl GooseberryTabs {
    /// Retrieve all entries from a folder (expects <entry_type>_<entry_id>.md)
    /// Make a tab for each kind of entry_type
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

    /// Renders the tab bar and calls the active tab's render function
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
            .style(Style::default().fg(CONFIG.tab_inactive_color))
            .highlight_style(Style::default().fg(CONFIG.tab_active_color));
        self.tabs[self.index].render(frame, &mut tabs);
    }

    /// Checks if the active tab is in writing mode
    pub fn is_writing(&self) -> bool {
        self.tabs[self.index].is_writing
    }

    /// Handle keyboard input events
    /// left and right arrow keys change the active tab
    /// `q` in scrolling mode returns true (to exit the app)
    /// Everything else is handled by the active tab's keypress function
    pub fn keypress(&mut self, terminal_size: Rect, key: KeyEvent) -> Result<bool, Error> {
        if !self.is_writing() {
            match key {
                KeyEvent::Char('q') => return Ok(true),
                KeyEvent::Right => self.next(),
                KeyEvent::Left => self.previous(),
                _key => self.tabs[self.index].keypress(terminal_size, _key)?,
            }
        } else {
            self.tabs[self.index].keypress(terminal_size, key)?;
        }
        Ok(false)
    }

    fn next(&mut self) {
        self.index = (self.index + 1) % self.tabs.len();
    }

    fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.tabs.len() - 1;
        }
    }
}

/// Tab for displaying and editing a list of entries
/// Also allows adding new ones
pub struct GooseberryTab {
    /// title of the Tab
    title: String,
    /// type of entries listed
    entry_type: entry::GooseberryEntryType,
    /// true => hides the longer descriptions
    fold: bool,
    /// dict of entry_id: entry
    entries: HashMap<u64, entry::GooseberryEntry>,
    /// which ids to display (TODO: use this when you add filtering options)
    visible_ids: Vec<u64>,
    /// true if Tab is in writing mode
    is_writing: bool,
    /// struct of text input boxes used in writing mode
    input_boxes: utility::interactive::InputBoxes,
    /// id to use when a new entry is added
    next_id: u64,
    /// folder in which entries are written
    folder: PathDir,
    /// scroll index for the list display
    scroll: u16,
    /// keeps track of the mode (editing/toggling task/deleting) (TODO: make this an enum)
    picking_char: Option<char>,
    /// true => Insert-Name-Here is currently selecting an ID
    picking_entry: bool,
    /// Entry ID entered
    selected_entry: u64,
    /// if editing an entry, this stores the old entry (TODO: add as a field to the `picking_char` enum)
    editing_entry: Option<entry::GooseberryEntry>,
    cursor: TerminalCursor,
}

//fn get_cursor(x: u16, y: u16) -> Result<(), Error> {
//
//    Ok(())
//}

impl GooseberryTab {
    /// retrieve entries of a given type from a given folder
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
            editing_entry: None,
            picking_entry: false,
            picking_char: None,
            cursor: cursor(),
        })
    }

    /// Makes the layout of the terminal based on the mode (writing/scrolling)
    fn get_layout(&self, terminal_size: Rect) -> Vec<Rect> {
        let constraints = if self.is_writing {
            self.input_boxes.get_constraints()
        } else {
            vec![
                Constraint::Percentage(TAB_BOX_PERCENT),
                Constraint::Percentage(100 - TAB_BOX_PERCENT - HELP_BOX_PERCENT),
                Constraint::Percentage(HELP_BOX_PERCENT),
            ]
        };
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(constraints.as_ref())
            .split(terminal_size)
    }

    /// Renders the help box at the bottom with the keyboard shortcuts
    /// Changes depending on the mode
    /// TODO: Add a small box here which displays what's being typed during ID entry mode
    fn render_help_box(&self, frame: &mut utility::interactive::TuiFrame, chunk: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title_style(Style::default().modifier(Modifier::BOLD));
        let text = if self.is_writing {
            WRITING_HELP_TEXT
        } else {
            HELP_TEXT
        };
        Paragraph::new(vec![Text::Raw(text.into())].iter())
            .block(block)
            .alignment(Alignment::Center)
            .wrap(true)
            .render(frame, chunk)
    }

    /// Renders the active tab
    /// Tab bar
    /// List of entries
    /// Help box
    /// if in writing mode then displays text input boxes
    pub fn render(&self, frame: &mut utility::interactive::TuiFrame, tabs: &mut Tabs<String>) {
        let chunks = self.get_layout(frame.size());
        tabs.render(frame, chunks[0]);
        Paragraph::new(
            entry::GooseberryEntry::entries_to_styled_texts_same_type(
                &self.entries,
                &self.visible_ids,
                self.fold,
                frame.size().width - 5,
            )
                .unwrap()
                .iter(),
        )
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Left)
            .scroll(self.scroll)
            .wrap(true)
            .render(frame, chunks[1]);
        if self.is_writing {
            self.input_boxes.render(&chunks[2..chunks.len() - 1], frame);
        }
        self.render_help_box(frame, chunks[chunks.len() - 1]);
    }

    /// Called when user inputs `t <id>[Enter]` in the Task tab
    /// toggles the state of a Task entry (done/not done)
    /// TODO: Restrict this to Task Tab
    fn toggle_task_entry(&mut self) -> Result<(), Error> {
        if self.entry_type == entry::GooseberryEntryType::Task {
            let t_entry =
                self.entries
                    .get_mut(&self.selected_entry)
                    .ok_or(Sorry::MissingEntryID {
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

    /// Handles keyboard input
    /// in scrolling mode:
    ///     ^ v: scrolls
    ///     n: starts/resumes writing mode
    ///     `\t`: toggles folding
    ///     e/t/d: starts ID entry mode
    ///     0-9: if in ID entry mode, adds the digit to `self.selected_entry`
    ///     `\n`: stops ID entry mode and executes e/t/d
    pub fn keypress(&mut self, terminal_size: Rect, key: KeyEvent) -> Result<(), Error> {
        if self.is_writing {
            let (new_entry, stop_writing) = self.input_boxes.keypress(
                &self.get_layout(terminal_size)[2..],
                &mut self.cursor,
                key,
            )?;
            if let Some(new_entry) = new_entry {
                if self.editing_entry.is_some() {
                    self.merge_entry(new_entry)?;
                    self.editing_entry = None;
                } else {
                    self.add_entry(new_entry, self.next_id)?;
                    self.next_id += 1;
                }
            }
            if stop_writing {
                self.is_writing = false;
                self.cursor.hide()?;
            }
        } else {
            match key {
                KeyEvent::Char(c) => match c {
                    'n' => {
                        self.is_writing = true;
                        self.input_boxes
                            .start_writing(&self.get_layout(terminal_size)[2..], &mut self.cursor)?;
                    }
                    '\t' => self.toggle_fold(),
                    't' | 'e' | 'd' => {
                        self.picking_char = Some(c);
                        self.picking_entry = true;
                        self.selected_entry = 0;
                    }
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
                                'e' => self.start_editing()?,
                                'd' => self.delete_entry(self.selected_entry)?,
                                _ => (),
                            }
                        }
                        self.picking_entry = false;
                        self.selected_entry = 0;
                        self.picking_char = None;
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
        Ok(())
    }

    /// fold = true => short display (title, date, tags)
    /// fold = false => displays everything
    pub fn toggle_fold(&mut self) {
        self.fold = !self.fold;
    }

    /// Put an existing entry into text input boxes for editing
    fn start_editing(&mut self) -> Result<(), Error> {
        let entry = self
            .entries
            .get(&self.selected_entry)
            .ok_or(Sorry::MissingEntryID {
                entry_type: self.entry_type,
                entry_id: self.selected_entry,
            })?
            .clone();
        self.input_boxes = entry.to_input_boxes();
        self.is_writing = true;
        self.editing_entry = Some(entry);
        Ok(())
    }

    /// Write entry to file
    fn save_entry(&self, id: u64) -> Result<(), Error> {
        self.entries
            .get(&id)
            .ok_or(Sorry::MissingEntryID {
                entry_type: self.entry_type,
                entry_id: id,
            })?
            .to_file(PathFile::create(
                self.entry_type.get_file(&self.folder, id)?,
            )?)?;
        Ok(())
    }

    /// Get an entry from input boxes after Ctrl-s in writing mode, merge it with the previous, save it to file
    fn merge_entry(&mut self, boxes: Vec<utility::interactive::InputBox>) -> Result<(), Error> {
        let editing_entry = self.editing_entry.as_ref().ok_or(Sorry::OutOfCheeseError {
            message: "I already checked that this is_some but is_none!".into(),
        })?;
        let id = editing_entry.id();
        let mut new_entry = entry::GooseberryEntry::from_input_boxes(id, self.entry_type, boxes)?;
        new_entry.merge_with_entry(editing_entry);
        self.entries.insert(id, new_entry);
        self.save_entry(id)?;
        Ok(())
    }

    /// Get an entry from input boxes after Ctrl-s in writing mode, save it to file
    fn add_entry(
        &mut self,
        boxes: Vec<utility::interactive::InputBox>,
        id: u64,
    ) -> Result<(), Error> {
        let new_entry = entry::GooseberryEntry::from_input_boxes(id, self.entry_type, boxes)?;
        self.entries.insert(id, new_entry);
        self.visible_ids.push(id);
        self.save_entry(id)?;
        Ok(())
    }

    /// Deletes an entry
    fn delete_entry(&mut self, id: u64) -> Result<(), Error> {
        if !self.entries.contains_key(&id) {
            return Err(Sorry::MissingEntryID {
                entry_type: self.entry_type,
                entry_id: id,
            }
                .into());
        }
        self.entries.remove(&id);
        self.visible_ids.remove_item(&id);
        self.entry_type.get_file(&self.folder, id)?.remove()?;
        Ok(())
    }
}
