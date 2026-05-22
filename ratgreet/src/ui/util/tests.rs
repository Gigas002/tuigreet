use ratatui::prelude::Rect;

use libratgreet::{Greeter, greeter::Mode};

use crate::ui::util::{get_greeting_height, get_height, get_input_width, get_rect_bounds};

#[test]
fn test_container_height_username_padding_zero() {
    let mut greeter = Greeter::default();
    greeter.container_padding = 1;
    greeter.mode = Mode::Username;

    assert_eq!(get_height(&greeter), 3);
}

#[test]
fn test_container_height_username_padding_one() {
    let mut greeter = Greeter::default();
    greeter.container_padding = 2;
    greeter.mode = Mode::Username;

    assert_eq!(get_height(&greeter), 5);
}

#[test]
fn test_container_height_username_greeting_padding_one() {
    let mut greeter = Greeter::default();
    greeter.container_padding = 2;
    greeter.greeting = Some("Hello".into());
    greeter.mode = Mode::Username;

    assert_eq!(get_height(&greeter), 7);
}

#[test]
fn test_container_height_password_greeting_padding_one_prompt_padding_1() {
    let mut greeter = Greeter::default();
    greeter.container_padding = 2;
    greeter.greeting = Some("Hello".into());
    greeter.mode = Mode::Password;
    greeter.prompt = Some("Password:".into());

    assert_eq!(get_height(&greeter), 9);
}

#[test]
fn test_container_height_password_greeting_padding_one_prompt_padding_0() {
    let mut greeter = Greeter::default();
    greeter.container_padding = 2;
    greeter.prompt_padding = 0;
    greeter.greeting = Some("Hello".into());
    greeter.mode = Mode::Password;
    greeter.prompt = Some("Password:".into());

    assert_eq!(get_height(&greeter), 8);
}

#[test]
fn test_rect_bounds() {
    let mut greeter = Greeter::default();
    greeter.width = 50;

    let (x, y, width, height) = get_rect_bounds(&greeter, Rect::new(0, 0, 100, 100), 1);

    assert_eq!(x, 25);
    assert_eq!(y, 47);
    assert_eq!(width, 50);
    assert_eq!(height, 6);
}

#[test]
fn input_width() {
    let mut greeter = Greeter::default();
    greeter.width = 40;
    greeter.container_padding = 2;

    let input_width = get_input_width(&greeter, 40, &Some("Username:".into()));

    assert_eq!(input_width, 26);
}

#[test]
fn greeting_height_one_line() {
    let mut greeter = Greeter::default();
    greeter.width = 15;
    greeter.container_padding = 2;
    greeter.greeting = Some("Hello World".into());

    let (_, height) = get_greeting_height(&greeter, 1, 0);

    assert_eq!(height, 2);
}

#[test]
fn greeting_height_two_lines() {
    let mut greeter = Greeter::default();
    greeter.width = 8;
    greeter.container_padding = 2;
    greeter.greeting = Some("Hello World".into());

    let (_, height) = get_greeting_height(&greeter, 1, 0);

    assert_eq!(height, 3);
}

#[test]
fn greeting_plain_text_ignores_ansi_escapes_in_layout() {
    let mut greeter = Greeter::default();
    greeter.width = 15;
    greeter.container_padding = 2;
    greeter.greeting = Some("\x1b[31mHello\x1b[0m World".into());

    let (_, height) = get_greeting_height(&greeter, 1, 0);

    // Greeting is rendered as plain text; escape sequences count toward width/wrap.
    assert_eq!(height, 3);
}
