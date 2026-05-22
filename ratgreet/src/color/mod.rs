//! CSS-like color strings for theme configuration (`#rgb`, `#rrggbb`, `#rrggbbaa`).

use std::str::FromStr;

use ratatui::style::Color;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseColorError;

impl std::fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid color")
    }
}

impl std::error::Error for ParseColorError {}

/// Parses a theme color: named ANSI colors (ratatui) or `#` hex like CSS.
///
/// Supports `#rgb`, `#rgba`, `#rrggbb`, and `#rrggbbaa`. The alpha channel is
/// accepted for config compatibility but ignored when rendering (ratatui has no
/// terminal alpha).
pub fn parse_css_color(s: &str) -> Result<Color, ParseColorError> {
    let s = s.trim();
    if s.starts_with('#') {
        parse_hex_color(s)
    } else {
        Color::from_str(s).map_err(|_| ParseColorError)
    }
}

fn parse_hex_color(s: &str) -> Result<Color, ParseColorError> {
    let hex = s.get(1..).ok_or(ParseColorError)?;
    let (r, g, b) = match hex.len() {
        3 => expand_nibble_triplet(hex)?,
        4 => {
            let (r, g, b) = expand_nibble_triplet(&hex[..3])?;
            let _a = hex_nibble(hex.as_bytes()[3])?;
            (r, g, b)
        }
        6 => parse_byte_triplet(hex)?,
        8 => {
            let (r, g, b) = parse_byte_triplet(&hex[..6])?;
            let _a = hex_byte(&hex[6..8])?;
            (r, g, b)
        }
        _ => return Err(ParseColorError),
    };
    Ok(Color::Rgb(r, g, b))
}

fn expand_nibble_triplet(hex: &str) -> Result<(u8, u8, u8), ParseColorError> {
    let bytes = hex.as_bytes();
    if bytes.len() != 3 {
        return Err(ParseColorError);
    }
    Ok((
        hex_nibble(bytes[0])? * 17,
        hex_nibble(bytes[1])? * 17,
        hex_nibble(bytes[2])? * 17,
    ))
}

fn parse_byte_triplet(hex: &str) -> Result<(u8, u8, u8), ParseColorError> {
    if hex.len() != 6 {
        return Err(ParseColorError);
    }
    Ok((
        hex_byte(&hex[0..2])?,
        hex_byte(&hex[2..4])?,
        hex_byte(&hex[4..6])?,
    ))
}

fn hex_nibble(byte: u8) -> Result<u8, ParseColorError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseColorError),
    }
}

fn hex_byte(hex: &str) -> Result<u8, ParseColorError> {
    let bytes = hex.as_bytes();
    if bytes.len() != 2 {
        return Err(ParseColorError);
    }
    Ok(hex_nibble(bytes[0])? * 16 + hex_nibble(bytes[1])?)
}
