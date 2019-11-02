use chrono::{DateTime, Utc};
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
fn style_title(id: u64, title: &str, mark: Option<TaskState>) -> Vec<Text> {
    let mut texts = vec![Text::raw(format!("{} ", id))];
    if let Some(state) = mark {
        texts.push(state.styled_symbol());
    }
    texts.push(Text::styled(
        format!("{}\n", title.trim()),
        TuiStyle::default().modifier(Modifier::ITALIC),
    ));
    texts
}

/// Style the date and time
fn style_datetime(datetime: &DateTime<Utc>) -> Text {
    Text::styled(
        format!("{}\n", datetime.format("%v %r")),
        TuiStyle::default().fg(CONFIG.datetime_color),
    )
}

fn style_tags(tags: &[String]) -> Text {
    Text::styled(
        format!("{}\n", tags.join(", ")),
        TuiStyle::default().fg(CONFIG.tags_color),
    )
}

pub fn style_people(people: &[String]) -> Text {
    Text::styled(
        format!("{}\n", people.join(", ")),
        TuiStyle::default()
            .fg(CONFIG.people_color)
    )
}

/// Style an entry for short display
/// ID <task state> Title
/// Date Time
/// Tags
pub fn style_short<'a>(
    id: u64,
    title: &'a str,
    mark: Option<TaskState>,
    datetime: &'a DateTime<Utc>,
    tags: &'a [String],
) -> Vec<Text<'a>> {
    let mut texts = style_title(id, title, mark);
    texts.push(style_datetime(datetime));
    texts.push(style_tags(tags));
    texts
}

/// Add a fake cursor
/// Couldn't figure out how to get the real cursor where we need it
pub fn cursor<'a>() -> Text<'a> {
    Text::Styled(
        CONFIG.cursor_char.to_string().into(),
        TuiStyle::default()
            .fg(CONFIG.cursor_color)
            .modifier(Modifier::BOLD),
    )
}
