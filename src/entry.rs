use std::{collections::HashMap, iter::Peekable, str::FromStr};

use anyhow::Error;
use chrono::{Date, DateTime, NaiveDateTime, Utc};
use path_abs::PathFile;
use tui::widgets::Text;

use crate::errors::Sorry;
use crate::utility;

#[derive(Debug)]
pub enum GooseberryEntry {
    Task(TaskEntry),
    Journal(JournalEntry),
    Research(ResearchEntry),
    Event(EventEntry),
}

impl GooseberryEntry {
    pub fn from_file(filename: &PathFile) -> Result<Self, Error> {
        let (entry_type, header, lines) = get_type_header_lines(filename)?;
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
}

pub trait GooseberryEntryTrait: Sized {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error>;
    fn id(&self) -> u64;
    fn tags(&self) -> &[u64];
    fn datetime(&self) -> &DateTime<Utc>;
}

pub trait GooseberryEntryFormat: GooseberryEntryTrait {
    fn to_file(&self, filename: PathFile) -> Result<(), Error>;
    fn to_tui_short(&self) -> Result<Vec<Text>, Error>;
    fn to_tui_long(&self) -> Result<Vec<Text>, Error>;
    fn format_id_datetime_tags(&self) -> String {
        format!(
            "ID: {}\nDateTime: {:?}\nTags: {}",
            self.id(),
            self.datetime(),
            self.tags()
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
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

#[derive(Copy, Debug, Clone)]
pub enum GooseberryEntryType {
    Task,
    Research,
    Journal,
    Event,
}

impl FromStr for GooseberryEntryType {
    type Err = Error;

    fn from_str(s: &str) -> Result<GooseberryEntryType, Error> {
        match s {
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

pub fn get_type_header_lines(
    filename: &PathFile,
) -> Result<(GooseberryEntryType, HashMap<String, String>, String), Error> {
    let content = filename.read_string()?;
    let mut lines = content.split('\n').peekable();
    let header = consume_markdown_header(&mut lines)?;
    let lines: String = lines.collect::<Vec<_>>().join("\n");
    let entry_type = (&header.get("Type").ok_or(Sorry::MissingHeaderElement {
        element: "Type".into(),
    })?)
        .parse::<GooseberryEntryType>()?;
    Ok((entry_type, header, lines))
}

fn get_id_datetime_tags(
    header: &HashMap<String, String>,
) -> Result<(u64, DateTime<Utc>, Vec<u64>), Error> {
    let id = header
        .get("ID")
        .ok_or(Sorry::MissingHeaderElement {
            element: "ID".into(),
        })?
        .parse::<u64>()?;
    let datetime = DateTime::from_utc(
        NaiveDateTime::parse_from_str(
            header.get("DateTime").ok_or(Sorry::MissingHeaderElement {
                element: "DateTime".into(),
            })?,
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
        .map(|t| t.parse::<u64>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok((id, datetime, tags))
}

#[derive(Clone, Debug)]
pub struct TaskEntry {
    pub id: u64,
    pub task: String,
    pub description: String,
    pub datetime: DateTime<Utc>,
    pub done: bool,
    pub tags: Vec<u64>,
}

impl TaskEntry {
    fn toggle(&mut self) {
        self.done = !self.done;
    }

    fn done(&mut self) {
        self.done = true;
    }

    fn not_done(&mut self) {
        self.done = false;
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

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[u64] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }
}

impl GooseberryEntryFormat for TaskEntry {
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
            utility::formatting::DONE
        } else {
            utility::formatting::NOT_DONE
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
    pub tags: Vec<u64>,
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

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[u64] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }
}

impl GooseberryEntryFormat for JournalEntry {
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
        let styled_text = self.to_tui_short()?;
        Ok(styled_text)
    }
}

#[derive(Clone, Debug)]
pub struct ResearchEntry {
    pub id: u64,
    pub title: String,
    pub notes: String,
    pub datetime: DateTime<Utc>,
    pub tags: Vec<u64>,
}

impl GooseberryEntryTrait for ResearchEntry {
    fn from_header_lines(header: HashMap<String, String>, lines: String) -> Result<Self, Error> {
        let (id, datetime, tags) = get_id_datetime_tags(&header)?;
        let title = header
            .get("Title")
            .ok_or(Sorry::MissingHeaderElement {
                element: "Title".into(),
            })?
            .to_owned();
        Ok(ResearchEntry {
            id,
            title,
            notes: lines,
            datetime,
            tags,
        })
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[u64] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }
}

impl GooseberryEntryFormat for ResearchEntry {
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
        styled_text.extend_from_slice(&utility::formatting::markdown_to_styled_texts(&self.notes));
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
    pub tags: Vec<u64>,
}

impl EventEntry {
    fn format_people(&self) -> String {
        self.people.join(",")
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

    fn id(&self) -> u64 {
        self.id
    }

    fn tags(&self) -> &[u64] {
        &self.tags
    }

    fn datetime(&self) -> &DateTime<Utc> {
        &self.datetime
    }
}

impl GooseberryEntryFormat for EventEntry {
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
        styled_text.extend_from_slice(&utility::formatting::markdown_to_styled_texts(&self.notes));
        Ok(styled_text)
    }
}
