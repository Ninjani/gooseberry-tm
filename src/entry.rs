use std::{collections::HashMap, fmt, iter::Peekable, str::FromStr};

use anyhow::Error;
use chrono::{Date, DateTime, NaiveDateTime, Utc};
use path_abs::{PathDir, PathFile, PathOps};
use tui::widgets::Text;

use crate::errors::Sorry;
use crate::utility::{
    self,
    interactive::{InputBox, InputBoxes},
};

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum GooseberryEntryType {
    Task,
    Research,
    Journal,
    Event,
}

impl GooseberryEntryType {
    pub fn get_file(&self, folder: &PathDir, id: u64) -> Result<PathFile, Error> {
        Ok(PathFile::create(
            folder.join(&format!("{}_{}.md", self, id)),
        )?)
    }
}

impl FromStr for GooseberryEntryType {
    type Err = Error;

    fn from_str(s: &str) -> Result<GooseberryEntryType, Error> {
        match s.trim() {
            "Task" => Ok(GooseberryEntryType::Task),
            "Research" => Ok(GooseberryEntryType::Research),
            "Journal" => Ok(GooseberryEntryType::Journal),
            "Event" => Ok(GooseberryEntryType::Event),
            _ => Err(Sorry::UnknownEntryType {
                entry_type: s.to_owned(),
            }
                .into()),
        }
    }
}

impl fmt::Display for GooseberryEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GooseberryEntryType::Task => write!(f, "Task"),
            GooseberryEntryType::Journal => write!(f, "Journal"),
            GooseberryEntryType::Research => write!(f, "Research"),
            GooseberryEntryType::Event => write!(f, "Event"),
        }
    }
}

pub trait GooseberryEntryTrait: Sized {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error>;
    fn from_input_boxes(
        id: u64,
        entry_type: GooseberryEntryType,
        boxes: Vec<InputBox>,
    ) -> Result<Self, Error>;
    fn to_input_boxes(&self) -> InputBoxes;
    fn id(&self) -> u64;
    fn tags(&self) -> &[String];
    fn datetime(&self) -> &DateTime<Utc>;
    fn entry_type(&self) -> GooseberryEntryType;
    fn to_file(&self, filename: PathFile) -> Result<(), Error>;
    fn to_tui_short(&self) -> Result<Vec<Text>, Error>;
    fn to_tui_long(&self) -> Result<Vec<Text>, Error>;
    fn format_id_datetime_tags(&self) -> String {
        format!(
            "Type: {}\nID: {}\nDateTime: {}\nTags: {}",
            self.entry_type(),
            self.id(),
            self.datetime().format("%v %r"),
            self.tags()
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug)]
pub enum GooseberryEntry {
    Task(TaskEntry),
    Journal(JournalEntry),
    Research(ResearchEntry),
    Event(EventEntry),
}

impl GooseberryEntry {
    pub fn from_file(filename: &PathFile) -> Result<Self, Error> {
        let (header, lines) = get_header_lines(filename)?;
        Self::from_header_lines(header, lines)
    }
}

impl GooseberryEntryTrait for GooseberryEntry {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error> {
        let entry_type = (&header.get("Type").ok_or(Sorry::MissingHeaderElement {
            element: "Type".into(),
        })?)
            .parse::<GooseberryEntryType>()?;
        match entry_type {
            GooseberryEntryType::Task => Ok(GooseberryEntry::Task(TaskEntry::from_header_lines(
                header, lines,
            )?)),
            GooseberryEntryType::Research => Ok(GooseberryEntry::Research(
                ResearchEntry::from_header_lines(header, lines)?,
            )),
            GooseberryEntryType::Journal => Ok(GooseberryEntry::Journal(
                JournalEntry::from_header_lines(header, lines)?,
            )),
            GooseberryEntryType::Event => Ok(GooseberryEntry::Event(
                EventEntry::from_header_lines(header, lines)?,
            )),
        }
    }
    fn from_input_boxes(
        id: u64,
        entry_type: GooseberryEntryType,
        boxes: Vec<InputBox>,
    ) -> Result<Self, Error> {
        match entry_type {
            GooseberryEntryType::Task => Ok(GooseberryEntry::Task(TaskEntry::from_input_boxes(
                id, entry_type, boxes,
            )?)),
            GooseberryEntryType::Journal => Ok(GooseberryEntry::Journal(
                JournalEntry::from_input_boxes(id, entry_type, boxes)?,
            )),
            GooseberryEntryType::Event => Ok(GooseberryEntry::Event(EventEntry::from_input_boxes(
                id, entry_type, boxes,
            )?)),
            GooseberryEntryType::Research => Ok(GooseberryEntry::Research(
                ResearchEntry::from_input_boxes(id, entry_type, boxes)?,
            )),
        }
    }

    fn to_input_boxes(&self) -> InputBoxes {
        match self {
            GooseberryEntry::Task(e) => e.to_input_boxes(),
            GooseberryEntry::Journal(e) => e.to_input_boxes(),
            GooseberryEntry::Event(e) => e.to_input_boxes(),
            GooseberryEntry::Research(e) => e.to_input_boxes(),
        }
    }

    fn id(&self) -> u64 {
        match self {
            GooseberryEntry::Task(e) => e.id(),
            GooseberryEntry::Journal(e) => e.id(),
            GooseberryEntry::Event(e) => e.id(),
            GooseberryEntry::Research(e) => e.id(),
        }
    }

    fn tags(&self) -> &[String] {
        match self {
            GooseberryEntry::Task(e) => e.tags(),
            GooseberryEntry::Journal(e) => e.tags(),
            GooseberryEntry::Event(e) => e.tags(),
            GooseberryEntry::Research(e) => e.tags(),
        }
    }

    fn datetime(&self) -> &DateTime<Utc> {
        match self {
            GooseberryEntry::Task(e) => e.datetime(),
            GooseberryEntry::Journal(e) => e.datetime(),
            GooseberryEntry::Event(e) => e.datetime(),
            GooseberryEntry::Research(e) => e.datetime(),
        }
    }

    fn entry_type(&self) -> GooseberryEntryType {
        match self {
            GooseberryEntry::Task(e) => e.entry_type(),
            GooseberryEntry::Journal(e) => e.entry_type(),
            GooseberryEntry::Event(e) => e.entry_type(),
            GooseberryEntry::Research(e) => e.entry_type(),
        }
    }

    fn to_file(&self, filename: PathFile) -> Result<(), Error> {
        match self {
            GooseberryEntry::Task(e) => e.to_file(filename),
            GooseberryEntry::Journal(e) => e.to_file(filename),
            GooseberryEntry::Event(e) => e.to_file(filename),
            GooseberryEntry::Research(e) => e.to_file(filename),
        }
    }

    fn to_tui_short(&self) -> Result<Vec<Text>, Error> {
        match self {
            GooseberryEntry::Task(e) => e.to_tui_short(),
            GooseberryEntry::Journal(e) => e.to_tui_short(),
            GooseberryEntry::Event(e) => e.to_tui_short(),
            GooseberryEntry::Research(e) => e.to_tui_short(),
        }
    }

    fn to_tui_long(&self) -> Result<Vec<Text>, Error> {
        match self {
            GooseberryEntry::Task(e) => e.to_tui_long(),
            GooseberryEntry::Journal(e) => e.to_tui_long(),
            GooseberryEntry::Event(e) => e.to_tui_long(),
            GooseberryEntry::Research(e) => e.to_tui_long(),
        }
    }
}

fn consume_markdown_header<'a>(
    lines: &mut Peekable<impl Iterator<Item = &'a str>>,
) -> Result<HashMap<String, String>, Error> {
    if lines.next().unwrap() != utility::formatting::HEADER_MARK {
        Err(Sorry::MissingHeader.into())
    } else {
        let mut header_lines = Vec::new();
        loop {
            if let Some(line) = lines.peek() {
                if line == &utility::formatting::HEADER_MARK {
                    lines.next().unwrap();
                    break;
                }
            }
            header_lines.push(lines.next().unwrap());
        }
        Ok(header_lines
            .into_iter()
            .map(|line| {
                let parts = line.split(": ").collect::<Vec<_>>();
                (parts[0].to_owned(), parts[1].to_owned())
            })
            .collect())
    }
}

impl GooseberryEntryType {
    pub fn get_input_boxes(&self) -> InputBoxes {
        match self {
            GooseberryEntryType::Task => InputBoxes::new(vec![
                InputBox::new(String::from("Task"), false, 10),
                InputBox::new(String::from("Description"), true, 60),
                InputBox::new(String::from("Tags"), false, 10),
            ]),
            GooseberryEntryType::Journal => InputBoxes::new(vec![
                InputBox::new(String::from("Description"), false, 10),
                InputBox::new(String::from("Tags"), false, 10),
            ]),
            GooseberryEntryType::Research => InputBoxes::new(vec![
                InputBox::new(String::from("Title"), false, 10),
                InputBox::new(String::from("Notes"), true, 60),
                InputBox::new(String::from("Tags"), false, 10),
            ]),
            GooseberryEntryType::Event => InputBoxes::new(vec![
                InputBox::new(String::from("Title"), false, 10),
                InputBox::new(String::from("Notes"), true, 50),
                InputBox::new(String::from("People"), false, 10),
                InputBox::new(String::from("Tags"), false, 10),
            ]),
        }
    }
}

pub fn get_header_lines(filename: &PathFile) -> Result<(HashMap<String, String>, String), Error> {
    let content = filename.read_string()?;
    let mut lines = content.split('\n').peekable();
    let header = consume_markdown_header(&mut lines)?;
    let lines: String = lines.collect::<Vec<_>>().join("\n");
    Ok((header, lines))
}

fn get_id_datetime_tags(
    header: &HashMap<String, String>,
) -> Result<(u64, DateTime<Utc>, Vec<String>), Error> {
    let id = header
        .get("ID")
        .ok_or(Sorry::MissingHeaderElement {
            element: "ID".into(),
        })?
        .parse::<u64>()?;
    let datetime = DateTime::from_utc(
        NaiveDateTime::parse_from_str(
            header
                .get("DateTime")
                .ok_or(Sorry::MissingHeaderElement {
                    element: "DateTime".into(),
                })?
                .trim(),
            "%v %r",
        )?,
        Utc,
    );
    let tags = header
        .get("Tags")
        .ok_or(Sorry::MissingHeaderElement {
            element: "Tags".into(),
        })?
        .split(',')
        .map(|t| t.trim().to_owned())
        .collect::<Vec<_>>();
    Ok((id, datetime, tags))
}

#[derive(Clone, Debug)]
pub struct TaskEntry {
    pub id: u64,
    pub task: String,
    pub description: String,
    pub datetime: DateTime<Utc>,
    pub done: bool,
    pub tags: Vec<String>,
}

impl TaskEntry {
    pub fn toggle(&mut self) {
        self.done = !self.done;
    }
}

impl GooseberryEntryTrait for TaskEntry {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error> {
        let (id, datetime, tags) = get_id_datetime_tags(&header)?;
        let task = header
            .get("Task")
            .ok_or(Sorry::MissingHeaderElement {
                element: "Task".into(),
            })?
            .trim()
            .to_owned();
        let done = header
            .get("Done")
            .ok_or(Sorry::MissingHeaderElement {
                element: "Done".into(),
            })?
            .trim()
            .parse::<bool>()?;
        Ok(TaskEntry {
            id,
            task,
            description: lines,
            datetime,
            done,
            tags,
        })
    }

    fn from_input_boxes(
        id: u64,
        entry_type: GooseberryEntryType,
        boxes: Vec<InputBox>,
    ) -> Result<Self, Error> {
        if entry_type != GooseberryEntryType::Task {
            return Err(Sorry::WrongEntryType {
                expected: GooseberryEntryType::Task,
                got: entry_type,
            }
                .into());
        }
        let (task, description) = (boxes[0].get_content(), boxes[1].get_content());
        let tags = boxes[2]
            .get_content()
            .split(',')
            .map(|t| t.trim().to_owned())
            .collect();
        Ok(TaskEntry {
            id,
            task,
            description,
            datetime: Utc::now(),
            done: false,
            tags,
        })
    }

    fn to_input_boxes(&self) -> InputBoxes {
        let mut input_boxes = self.entry_type().get_input_boxes();
        input_boxes.replace_content(0, &self.task);
        input_boxes.replace_content(1, &self.description);
        input_boxes.replace_content(2, &self.tags.join(", "));
        input_boxes
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[String] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }

    fn entry_type(&self) -> GooseberryEntryType {
        GooseberryEntryType::Task
    }

    fn to_file(&self, filename: PathFile) -> Result<(), Error> {
        let header = format!(
            "{}\n{}\nTask: {}\nDone: {}\n{}\n",
            utility::formatting::HEADER_MARK,
            self.format_id_datetime_tags(),
            self.task,
            self.done,
            utility::formatting::HEADER_MARK,
        );
        filename.write_str(&format!("{}{}", header, self.description))?;
        Ok(())
    }

    fn to_tui_short(&self) -> Result<Vec<Text>, Error> {
        let mark = if self.done {
            utility::formatting::TaskState::Done
        } else {
            utility::formatting::TaskState::NotDone
        };
        Ok(utility::formatting::style_short(
            self.id,
            &self.task,
            Some(mark),
            &self.datetime,
            &self.tags,
        ))
    }

    fn to_tui_long(&self) -> Result<Vec<Text>, Error> {
        let mut styled_text = self.to_tui_short()?;
        styled_text.extend_from_slice(&utility::formatting::markdown_to_styled_texts(
            &self.description,
        ));
        styled_text.push(Text::Raw("\n---\n".into()));
        Ok(styled_text)
    }
}

#[derive(Clone, Debug)]
pub struct JournalEntry {
    pub id: u64,
    pub description: String,
    pub datetime: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl JournalEntry {
    fn date(&self) -> Date<Utc> {
        self.datetime.date()
    }
}

impl GooseberryEntryTrait for JournalEntry {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error> {
        let (id, datetime, tags) = get_id_datetime_tags(&header)?;
        Ok(JournalEntry {
            id,
            description: lines,
            datetime,
            tags,
        })
    }

    fn from_input_boxes(
        id: u64,
        entry_type: GooseberryEntryType,
        boxes: Vec<InputBox>,
    ) -> Result<Self, Error> {
        if entry_type != GooseberryEntryType::Journal {
            return Err(Sorry::WrongEntryType {
                expected: GooseberryEntryType::Journal,
                got: entry_type,
            }
                .into());
        }
        let description = boxes[0].get_content();
        let tags = boxes[1]
            .get_content()
            .split(',')
            .map(|t| t.trim().to_owned())
            .collect();

        Ok(JournalEntry {
            id,
            description,
            datetime: Utc::now(),
            tags,
        })
    }

    fn to_input_boxes(&self) -> InputBoxes {
        let mut input_boxes = self.entry_type().get_input_boxes();
        input_boxes.replace_content(0, &self.description);
        input_boxes.replace_content(1, &self.tags.join(", "));
        input_boxes
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[String] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }

    fn entry_type(&self) -> GooseberryEntryType {
        GooseberryEntryType::Journal
    }

    fn to_file(&self, filename: PathFile) -> Result<(), Error> {
        let header = format!(
            "{}\n{}\n{}\n",
            utility::formatting::HEADER_MARK,
            self.format_id_datetime_tags(),
            utility::formatting::HEADER_MARK
        );
        filename.write_str(&format!("{}{}", header, self.description))?;
        Ok(())
    }

    fn to_tui_short(&self) -> Result<Vec<Text>, Error> {
        Ok(utility::formatting::style_short(
            self.id,
            &self.description,
            None,
            &self.datetime,
            &self.tags,
        ))
    }

    fn to_tui_long(&self) -> Result<Vec<Text>, Error> {
        let mut styled_text = self.to_tui_short()?;
        styled_text.push(Text::Raw("\n---\n".into()));
        Ok(styled_text)
    }
}

#[derive(Clone, Debug)]
pub struct ResearchEntry {
    pub id: u64,
    pub title: String,
    pub notes: String,
    pub datetime: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl GooseberryEntryTrait for ResearchEntry {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error> {
        let (id, datetime, tags) = get_id_datetime_tags(&header)?;
        let title = header
            .get("Title")
            .ok_or(Sorry::MissingHeaderElement {
                element: "Title".into(),
            })?
            .trim()
            .to_owned();
        Ok(ResearchEntry {
            id,
            title,
            notes: lines,
            datetime,
            tags,
        })
    }

    fn from_input_boxes(
        id: u64,
        entry_type: GooseberryEntryType,
        boxes: Vec<InputBox>,
    ) -> Result<Self, Error> {
        if entry_type != GooseberryEntryType::Research {
            return Err(Sorry::WrongEntryType {
                expected: GooseberryEntryType::Research,
                got: entry_type,
            }
                .into());
        }
        let (title, notes) = (boxes[0].get_content(), boxes[1].get_content());
        let tags = boxes[2]
            .get_content()
            .split(',')
            .map(|t| t.trim().to_owned())
            .collect();
        Ok(ResearchEntry {
            id,
            title,
            notes,
            datetime: Utc::now(),
            tags,
        })
    }

    fn to_input_boxes(&self) -> InputBoxes {
        let mut input_boxes = self.entry_type().get_input_boxes();
        input_boxes.replace_content(0, &self.title);
        input_boxes.replace_content(1, &self.notes);
        input_boxes.replace_content(2, &self.tags.join(", "));
        input_boxes
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[String] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }

    fn entry_type(&self) -> GooseberryEntryType {
        GooseberryEntryType::Research
    }

    fn to_file(&self, filename: PathFile) -> Result<(), Error> {
        let header = format!(
            "{}\n{}\nTitle: {}\n{}\n",
            utility::formatting::HEADER_MARK,
            self.format_id_datetime_tags(),
            self.title,
            utility::formatting::HEADER_MARK,
        );
        filename.write_str(&format!("{}{}", header, self.notes))?;
        Ok(())
    }

    fn to_tui_short(&self) -> Result<Vec<Text>, Error> {
        Ok(utility::formatting::style_short(
            self.id,
            &self.title,
            None,
            &self.datetime,
            &self.tags,
        ))
    }

    fn to_tui_long(&self) -> Result<Vec<Text>, Error> {
        let mut styled_text = self.to_tui_short()?;
        styled_text.extend_from_slice(&utility::formatting::markdown_to_styled_texts(
            &self.notes.trim(),
        ));
        styled_text.push(Text::raw("\n---"));
        Ok(styled_text)
    }
}

#[derive(Clone, Debug)]
pub struct EventEntry {
    pub id: u64,
    pub title: String,
    pub people: Vec<String>,
    pub datetime: DateTime<Utc>,
    pub notes: String,
    pub tags: Vec<String>,
}

impl EventEntry {
    fn format_people(&self) -> String {
        self.people.join(", ")
    }
}

impl GooseberryEntryTrait for EventEntry {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error> {
        let (id, datetime, tags) = get_id_datetime_tags(&header)?;
        let title = header
            .get("Title")
            .ok_or(Sorry::MissingHeaderElement {
                element: "Title".into(),
            })?
            .trim()
            .to_owned();
        let people = header
            .get("People")
            .ok_or(Sorry::MissingHeaderElement {
                element: "People".into(),
            })?
            .split(',')
            .map(|p| p.trim().to_owned())
            .collect();
        Ok(EventEntry {
            id,
            title,
            people,
            datetime,
            notes: lines,
            tags,
        })
    }

    fn from_input_boxes(
        id: u64,
        entry_type: GooseberryEntryType,
        boxes: Vec<InputBox>,
    ) -> Result<Self, Error> {
        if entry_type != GooseberryEntryType::Event {
            return Err(Sorry::WrongEntryType {
                expected: GooseberryEntryType::Event,
                got: entry_type,
            }
                .into());
        }
        let (title, notes) = (boxes[0].get_content(), boxes[1].get_content());
        let people = boxes[2]
            .get_content()
            .split(',')
            .map(|t| t.trim().to_owned())
            .collect();
        let tags = boxes[3]
            .get_content()
            .split(',')
            .map(|t| t.trim().to_owned())
            .collect();
        Ok(EventEntry {
            id,
            title,
            notes,
            datetime: Utc::now(),
            people,
            tags,
        })
    }

    fn to_input_boxes(&self) -> InputBoxes {
        let mut input_boxes = self.entry_type().get_input_boxes();
        input_boxes.replace_content(0, &self.title);
        input_boxes.replace_content(1, &self.notes);
        input_boxes.replace_content(2, &self.people.join(", "));
        input_boxes.replace_content(2, &self.tags.join(", "));
        input_boxes
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[String] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }

    fn entry_type(&self) -> GooseberryEntryType {
        GooseberryEntryType::Event
    }

    fn to_file(&self, filename: PathFile) -> Result<(), Error> {
        let header = format!(
            "{}\n{}\nTitle: {}\nPeople: {}\n{}\n",
            utility::formatting::HEADER_MARK,
            self.format_id_datetime_tags(),
            self.title,
            self.format_people(),
            utility::formatting::HEADER_MARK,
        );
        filename.write_str(&format!("{}{}", header, self.notes))?;
        Ok(())
    }

    fn to_tui_short(&self) -> Result<Vec<Text>, Error> {
        Ok(utility::formatting::style_short(
            self.id,
            &self.title,
            None,
            &self.datetime,
            &self.tags,
        ))
    }

    fn to_tui_long(&self) -> Result<Vec<Text>, Error> {
        let mut styled_text = self.to_tui_short()?;
        styled_text.push(utility::formatting::style_people(&self.people));
        styled_text.extend_from_slice(&utility::formatting::markdown_to_styled_texts(
            &self.notes.trim(),
        ));
        styled_text.push(Text::Raw("\n---\n".into()));
        Ok(styled_text)
    }
}
