use std::collections::HashMap;

use ratatui::style::Color;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Colors {
    #[serde(default = "default_bg", deserialize_with = "prefix_hex_code")]
    pub bg: Color,

    #[serde(default = "default_fg", deserialize_with = "prefix_hex_code")]
    pub fg: Color,

    #[serde(default = "default_color", deserialize_with = "prefix_hex_code")]
    pub accent: Color,

    #[serde(default = "default_color", deserialize_with = "prefix_hex_code")]
    pub primary: Color,

    #[serde(default = "default_success", deserialize_with = "prefix_hex_code")]
    pub success: Color,

    #[serde(default = "default_error", deserialize_with = "prefix_hex_code")]
    pub error: Color,

    #[serde(default = "default_warning", deserialize_with = "prefix_hex_code")]
    pub warning: Color,

    #[serde(default = "default_color", deserialize_with = "prefix_hex_code")]
    pub info: Color,

    #[serde(default = "default_color", deserialize_with = "prefix_hex_code")]
    pub title: Color,

    #[serde(default = "default_color", deserialize_with = "prefix_hex_code")]
    pub border: Color,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub colors: HashMap<String, Colors>,
}

fn default_bg() -> Color {
    Color::Reset
}

fn default_fg() -> Color {
    Color::White
}

fn default_color() -> Color {
    Color::DarkGray
}

fn default_success() -> Color {
    Color::Green
}

fn default_error() -> Color {
    Color::Red
}

fn default_warning() -> Color {
    Color::Yellow
}

fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Color::White; // fallback
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

    Color::Rgb(r, g, b)
}

fn prefix_hex_code<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(hex_to_color(&s))
}
