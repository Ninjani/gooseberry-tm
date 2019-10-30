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

pub const HEADER_MARK: &str = "---";
pub const DONE: char = '\u{2713}';
pub const NOT_DONE: char = '\u{2715}';

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
    static ref THEME: &'static Theme = &THEME_SET.themes["base16-ocean.dark"];
    static ref MD_SYNTAX: &'static SyntaxReference =
        SYNTAX_SET.find_syntax_by_extension("markdown").unwrap();
}

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

fn style_title(id: u64, title: &str, mark: Option<char>) -> Text {
    Text::styled(
        format!("{} {} {}\n", id, mark.unwrap_or(' '), title),
        TuiStyle::default()
            .fg(TuiColor::White)
            .modifier(Modifier::BOLD),
    )
}

fn style_datetime(datetime: &DateTime<Utc>) -> Text {
    Text::styled(
        format!("{}\n", datetime.format("%v %r")),
        TuiStyle::default().fg(TuiColor::LightBlue),
    )
}

fn style_tags(tags: &[u64]) -> Text {
    Text::styled(
        format!(
            "{}\n",
            tags.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TuiStyle::default().fg(TuiColor::LightGreen),
    )
}

pub fn style_short<'a>(
    id: u64,
    title: &'a str,
    mark: Option<char>,
    datetime: &'a DateTime<Utc>,
    tags: &'a [u64],
) -> Vec<Text<'a>> {
    vec![
        style_title(id, title, mark),
        style_datetime(datetime),
        style_tags(tags),
        Text::raw("\n"),
    ]
}

pub fn style_people(people: &[String]) -> Text {
    Text::styled(
        people.join(", "),
        TuiStyle::default().modifier(Modifier::BOLD),
    )
}

pub fn cursor<'a>() -> Text<'a> {
    Text::Styled(
        "|".into(),
        TuiStyle::default()
            .fg(TuiColor::LightYellow)
            .modifier(Modifier::BOLD),
    )
}
