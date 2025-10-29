use std::collections::HashMap;

use crossterm::style::Color;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Colors {
    #[serde(default = "default_bg", deserialize_with = "prefix_hex_code")]
    pub bg: Color,

    #[serde(default = "default_fg", deserialize_with = "prefix_hex_code")]
    pub fg: Color,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub colors: HashMap<String, Colors>,
}

fn default_bg() -> Color {
    Color::White
}

fn default_fg() -> Color {
    Color::White
}

fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Color::White; // fallback
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

    Color::Rgb { r, g, b }
}

fn prefix_hex_code<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(hex_to_color(&s))
}
