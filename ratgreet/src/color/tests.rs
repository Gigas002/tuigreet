use ratatui::style::Color;

use super::*;

#[test]
fn named_color() {
    assert_eq!(parse_css_color("blue").unwrap(), Color::Blue);
}

#[test]
fn hex_rgb_shorthand() {
    assert_eq!(parse_css_color("#f00").unwrap(), Color::Rgb(255, 0, 0));
}

#[test]
fn hex_rrggbb() {
    assert_eq!(parse_css_color("#FF0000").unwrap(), Color::Rgb(255, 0, 0));
}

#[test]
fn hex_rrggbbaa_ignores_alpha() {
    assert_eq!(
        parse_css_color("#ffffff00").unwrap(),
        Color::Rgb(255, 255, 255)
    );
}

#[test]
fn invalid_hex() {
    assert!(parse_css_color("#gggggg").is_err());
}
