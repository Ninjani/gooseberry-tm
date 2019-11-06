use chrono::{Date, DateTime, NaiveTime, Utc};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, Style as SyntectStyle, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};
use tui::{
    style::{Color as TuiColor, Modifier, Style as TuiStyle},
    widgets::Text,
};
use unicode_width::UnicodeWidthStr;

use crate::utility::config::CONFIG;

pub const HEADER_MARK: &str = "---";
pub const DONE: char = '\u{2713}';
pub const NOT_DONE: char = '\u{2715}';

/// Task states
/// TODO: I guess running/canceled can be some new ones
#[derive(Copy, Debug, Clone)]
pub enum TaskState {
    Done,
    NotDone,
}

impl TaskState {
    /// Unicode symbol for task states
    fn symbol(self) -> char {
        match self {
            TaskState::Done => '\u{2713}',
            TaskState::NotDone => '\u{2715}',
        }
    }

    /// Color for task states
    /// TODO: Make all colors configurable
    fn color(self) -> TuiColor {
        match self {
            TaskState::Done => TuiColor::Green,
            TaskState::NotDone => TuiColor::Red,
        }
    }

    /// Put the color onto the symbol
    fn styled_symbol<'a>(self) -> Text<'a> {
        Text::Styled(
            format!("{} ", self.symbol()).into(),
            TuiStyle::default().fg(self.color()),
        )
    }
}

lazy_static! {
    /// Load theme sets
    /// TODO: Save to file maybe?
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
    /// Load selected highlighting style
    static ref THEME: &'static Theme = &THEME_SET.themes[&CONFIG.syntax_theme];
    /// Load syntax sets
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    /// Load markdown syntax set
    static ref MD_SYNTAX: &'static SyntaxReference =
        SYNTAX_SET.find_syntax_by_extension("markdown").unwrap();
}

/// Convert from `syntect`'s FontStyle to `tui`'s Modifier
/// Reminder: `tui` doesn't have some of the options
fn syntect_to_tui_modifier(syntect_modifier: FontStyle) -> Modifier {
    let mut modifier = Modifier::empty();
    if syntect_modifier.contains(FontStyle::BOLD) {
        modifier |= Modifier::BOLD;
    }
    if syntect_modifier.contains(FontStyle::UNDERLINE) {
        modifier |= Modifier::UNDERLINED;
    }
    if syntect_modifier.contains(FontStyle::ITALIC) {
        modifier |= Modifier::ITALIC;
    }
    modifier
}

/// Convert a markdown-formatted string to a list of `tui` Text::styled objects
pub fn markdown_to_styled_texts(markdown_text: &str) -> Vec<Text> {
    let mut styled_texts = Vec::new();
    let mut highlighter = HighlightLines::new(&MD_SYNTAX, &THEME);
    for line in LinesWithEndings::from(&markdown_text) {
        for (syn_style, text) in highlighter.highlight(&line, &SYNTAX_SET) {
            styled_texts.push(Text::styled(
                text.to_string(),
                syntect_to_tui_style(syn_style),
            ));
        }
    }
    styled_texts
}

/// Add Modifier::DIM to each Text
fn dim(markdown: Vec<Text>) -> Vec<Text> {
    markdown
        .into_iter()
        .map(|styled_text| match styled_text {
            Text::Styled(text, mut style) => {
                style.modifier |= Modifier::DIM;
                Text::Styled(text, style)
            }
            Text::Raw(text) => Text::Styled(text, TuiStyle::default().modifier(Modifier::DIM)),
        })
        .collect()
}

/// Convert `syntect`'s Style to `tui`'s Style
fn syntect_to_tui_style(syntect_style: SyntectStyle) -> TuiStyle {
    TuiStyle {
        fg: TuiColor::Rgb(
            syntect_style.foreground.r,
            syntect_style.foreground.g,
            syntect_style.foreground.b,
        ),
        bg: TuiColor::Rgb(
            syntect_style.background.r,
            syntect_style.background.g,
            syntect_style.background.b,
        ),
        modifier: syntect_to_tui_modifier(syntect_style.font_style),
    }
}

/// Add Style to a title with optional Task state
fn style_title(id: u64, title: &str, mark: Option<TaskState>, terminal_width: u16, bold: bool) -> Vec<Text> {
    let mut texts = Vec::new();
    let mut terminal_width = terminal_width;
    if let Some(state) = mark {
        texts.push(state.styled_symbol());
        terminal_width -= 2;
    }
    let modifier = if bold {
        Modifier::ITALIC | Modifier::BOLD
    } else {
        Modifier::ITALIC
    };
    texts.push(Text::styled(
        right_format(title.trim(), &format!("{}", id), terminal_width, false),
        TuiStyle::default().modifier(modifier),
    ));
    texts
}

fn format_date(date: Date<Utc>) -> String {
    format!("{}", date.format("%b %d %Y"))
}

fn format_time(time: NaiveTime) -> String {
    format!("{}", time.format("%r"))
}

fn format_datetime(datetime: DateTime<Utc>) -> String {
    format!("{}", datetime.format("%r %a %b %d %Y"))
}

pub(crate) fn style_people(people: &[String]) -> Text {
    Text::styled(
        format!("{}\n", people.join(", ")),
        TuiStyle::default().fg(CONFIG.secondary_metadata_color),
    )
}

/// Style datetime and tags on same line, tags on left, date on right
fn style_datetime_tags<'a>(datetime: &'a DateTime<Utc>, tags: &'a [String], terminal_width: u16, date_only: bool, time_only: bool) -> Text<'a> {
    let datetime_formatted = if date_only {
        format_date(datetime.date())
    } else if time_only {
        format_time(datetime.time())
    } else {
        format_datetime(*datetime)
    };
    Text::styled(
        right_format(&tags.join(","), &datetime_formatted, terminal_width, true),
        TuiStyle::default().fg(CONFIG.primary_metadata_color),
    )
}

/// Style an entry for short display
/// ID <task state> Title
/// Date Time
/// Tags
pub(crate) fn style_short<'a>(
    id: u64,
    title: &'a str,
    mark: Option<TaskState>,
    datetime: &'a DateTime<Utc>,
    tags: &'a [String],
    terminal_width: u16,
    date_only: bool,
    time_only: bool,
    bold_title: bool
) -> Vec<Text<'a>> {
    let mut texts = style_title(id, title, mark, terminal_width, bold_title);
    texts.push(style_datetime_tags(datetime, tags, terminal_width, date_only, time_only));
    texts
}

pub(crate) fn style_date_num_entries<'a>(date: Date<Utc>, num_entries: usize, terminal_width: u16) -> Text<'a> {
    let entry_text = if num_entries > 1 { "entries" } else { "entry" };
    Text::styled(right_format(&format_date(date),
                              &format!("{} {}", num_entries, entry_text), terminal_width, true),
                 TuiStyle::default().fg(CONFIG.secondary_metadata_color).modifier(Modifier::BOLD))
}

/// Add a fake cursor
/// Couldn't figure out how to get the real cursor where we need it
pub(crate) fn cursor<'a>() -> Text<'a> {
    Text::Styled(
        CONFIG.cursor_char.to_string().into(),
        TuiStyle::default()
            .fg(CONFIG.cursor_color)
            .modifier(Modifier::BOLD),
    )
}

/// Adds text to an existing string but on the right. If there's not enough
/// space in the terminal to do that with at least one space in the middle
/// then puts the new_text on the next line (on the left if left_too_long else right)
fn right_format(text: &str, new_text: &str, terminal_width: u16, left_too_long: bool) -> String {
    let terminal_width = terminal_width as usize;
    let text_len = UnicodeWidthStr::width(text);
    let new_text_len = UnicodeWidthStr::width(new_text);
    if terminal_width < text_len + new_text_len + 1 {
        if left_too_long {
            format!("{}\n{}\n", text, new_text)
        } else {
            format!("{}\n{}", text, right_format("", new_text, terminal_width as u16, true))
        }

    } else {
        let num_spaces = terminal_width - text_len - new_text_len;
        format!(
            "{}{}{}\n",
            text,
            (0..num_spaces)
                .map(|_| " ".into())
                .collect::<Vec<String>>()
                .join(""),
            new_text
        )
    }
}
