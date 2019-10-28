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
use chrono::{Utc, DateTime};


pub const HEADER_MARK: &'static str = "---";
pub const DONE: char = '\u{2611}';
pub const NOT_DONE: char = '\u{2612}';
pub type Markdown<'a> = Vec<Text<'a>>;

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
    static ref THEME: &'static Theme = &THEME_SET.themes["base16-ocean.dark"];
    static ref MD_SYNTAX: &'static SyntaxReference =
        SYNTAX_SET.find_syntax_by_extension("markdown").unwrap();
}


fn syntect_to_tui_modifier(syntect_modifier: FontStyle) -> Modifier {
    let f_bui = FontStyle::BOLD | FontStyle::UNDERLINE | FontStyle::ITALIC;
    let f_bu = FontStyle::BOLD | FontStyle::UNDERLINE;
    let f_bi = FontStyle::BOLD | FontStyle::ITALIC;
    let f_ui = FontStyle::UNDERLINE | FontStyle::ITALIC;
    match syntect_modifier {
        FontStyle::BOLD => Modifier::BOLD,
        FontStyle::UNDERLINE => Modifier::UNDERLINED,
        FontStyle::ITALIC => Modifier::ITALIC,
        syn => {
            if syn == f_bui {
                (Modifier::BOLD | Modifier::UNDERLINED | Modifier::ITALIC)
            } else if syn == f_ui {
                (Modifier::ITALIC | Modifier::UNDERLINED)
            } else if syn == f_bu {
                (Modifier::BOLD | Modifier::UNDERLINED)
            } else if syn == f_bi {
                (Modifier::BOLD | Modifier::ITALIC)
            } else {
                Modifier::empty()
            }
        }
    }
}

pub fn markdown_to_styled_texts(markdown_text: &str) -> Markdown {
    let mut styled_texts = Vec::new();
    let mut highlighter = HighlightLines::new(&MD_SYNTAX, &THEME);
    for line in LinesWithEndings::from(&markdown_text) {
        for (syn_style, text) in highlighter.highlight(&line, &SYNTAX_SET) {
            styled_texts.push(Text::styled(text, syntect_to_tui_style(syn_style)));
        }
    }
    styled_texts
}


fn dim(markdown: Markdown) -> Markdown {
    markdown
        .into_iter()
        .map(|styled_text| match styled_text {
            Text::Styled(text, mut style) => {
                style.modifier |= Modifier::DIM;
                Text::Styled(text, style)
            }
            _ => styled_text,
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

fn style_title(title: &str, mark: Option<char>) -> Text {
    Text::styled(format!("{} {}", mark.unwrap_or(' '), title),
                 TuiStyle::default().fg(TuiColor::LightCyan))
}

fn style_datetime(datetime: &DateTime<Utc>) -> Text {
    Text::styled(format!("{}", datetime.format("%v %r")),TuiStyle::default().fg(TuiColor::Red).modifier(Modifier::DIM))
}

fn style_tags(tags: &[u64]) -> Text {
    Text::styled(tags.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "),
                 TuiStyle::default().fg(TuiColor::Gray).modifier(Modifier::DIM))
}

pub fn style_short<'a>(title: &'a str, mark: Option<char>, datetime: &'a DateTime<Utc>, tags: &'a [u64]) -> Markdown<'a> {
    vec![style_title(title, mark), style_datetime(datetime), style_tags(tags)]
}

pub fn style_people(people: &[String]) -> Text {
    Text::styled(people.join(", "), TuiStyle::default().modifier(Modifier::BOLD))
}