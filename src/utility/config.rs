use tui::style::Color;

lazy_static! {
    pub static ref CONFIG: GooseberryConfig = GooseberryConfig::default();
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(remote = "Color")]
enum GooseberryColor {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GooseberryConfig {
    pub syntax_theme: String,
    #[serde(with = "GooseberryColor")]
    pub primary_metadata_color: Color,
    #[serde(with = "GooseberryColor")]
    pub secondary_metadata_color: Color,
    pub cursor_char: char,
    #[serde(with = "GooseberryColor")]
    pub cursor_color: Color,
    #[serde(with = "GooseberryColor")]
    pub tab_inactive_color: Color,
    #[serde(with = "GooseberryColor")]
    pub tab_active_color: Color,
}

impl Default for GooseberryConfig {
    fn default() -> Self {
        Self {
            syntax_theme: "base16-ocean.dark".into(),
            primary_metadata_color: Color::Blue,
            secondary_metadata_color: Color::Green,
            cursor_char: '|',
            cursor_color: Color::Gray,
            tab_inactive_color: Color::LightGreen,
            tab_active_color: Color::Blue,
        }
    }
}
