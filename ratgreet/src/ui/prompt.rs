use std::error::Error;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::ui::{
    Frame,
    common::style::{Theme, Themed},
    prompt_value, strings, themed_text,
    util::*,
};
use libratgreet::{
    greeter::{GreetAlign, Greeter, Mode, SecretDisplay},
    info::get_hostname,
};

const GREETING_INDEX: usize = 0;
const USERNAME_INDEX: usize = 1;
const ANSWER_INDEX: usize = 3;

pub fn draw(
    greeter: &mut Greeter,
    theme: &Theme,
    f: &mut Frame,
) -> Result<(u16, u16), Box<dyn Error>> {
    let size = f.area();
    let (x, y, width, height) = get_rect_bounds(greeter, size, 0);

    let container_padding = greeter.container_padding();
    let prompt_padding = greeter.prompt_padding();
    let greeting_alignment = match greeter.greet_align() {
        GreetAlign::Center => Alignment::Center,
        GreetAlign::Left => Alignment::Left,
        GreetAlign::Right => Alignment::Right,
    };

    let container = Rect::new(x, y, width, height);
    let frame = Rect::new(
        x + container_padding,
        y + container_padding,
        width - (2 * container_padding),
        height - (2 * container_padding),
    );

    let hostname = Span::from(titleize(&strings::title_authenticate(&get_hostname())));
    let block = Block::default()
        .title(hostname)
        .title_style(theme.of(&[Themed::Title]))
        .style(theme.of(&[Themed::Container]))
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme.of(&[Themed::Border]));

    f.render_widget(block, container);

    let (message, message_height) = get_message_height(greeter, theme, container_padding, 1);
    let (greeting, greeting_height) = get_greeting_height(greeter, container_padding, 0);

    let should_display_answer = greeter.mode == Mode::Password;

    let constraints = [
        Constraint::Length(greeting_height), // Greeting
        Constraint::Length(1),               // Username
        Constraint::Length(if should_display_answer {
            prompt_padding
        } else {
            0
        }), // Prompt padding
        Constraint::Length(if should_display_answer { 1 } else { 0 }), // Answer
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.as_ref())
        .split(frame);
    let cursor = chunks[USERNAME_INDEX];

    if let Some(greeting) = greeting {
        let greeting_label = greeting
            .alignment(greeting_alignment)
            .style(theme.of(&[Themed::Greet]));

        f.render_widget(greeting_label, chunks[GREETING_INDEX]);
    }

    let username_text = prompt_value(theme, Some(strings::get("username")));
    let username_label = Paragraph::new(username_text);

    let username = greeter.username.get();
    let username_value_text = Span::from(username);
    let username_value = Paragraph::new(username_value_text).style(theme.of(&[Themed::Input]));

    match greeter.mode {
        Mode::Username | Mode::Password | Mode::Action => {
            f.render_widget(username_label, chunks[USERNAME_INDEX]);

            f.render_widget(
                username_value,
                Rect::new(
                    1 + chunks[USERNAME_INDEX].x + strings::get("username").chars().count() as u16,
                    chunks[USERNAME_INDEX].y,
                    get_input_width(greeter, width, &Some(strings::get("username"))),
                    1,
                ),
            );

            let answer_text = if greeter.working {
                themed_text(theme, strings::get("wait"))
            } else {
                prompt_value(theme, greeter.prompt.as_ref())
            };

            let answer_label = Paragraph::new(answer_text);

            if greeter.mode == Mode::Password || greeter.previous_mode == Mode::Password {
                f.render_widget(answer_label, chunks[ANSWER_INDEX]);

                if !greeter.asking_for_secret || greeter.secret_display.shows_input() {
                    let value = if greeter.asking_for_secret {
                        match greeter.secret_display {
                            SecretDisplay::Hidden => String::new(),
                            SecretDisplay::Plain => greeter.buffer.clone(),
                            SecretDisplay::Masked(c) => {
                                std::iter::repeat_n(c, greeter.buffer.chars().count()).collect()
                            }
                        }
                    } else {
                        greeter.buffer.clone()
                    };

                    let answer_value_text = Span::from(value);
                    let answer_value =
                        Paragraph::new(answer_value_text).style(theme.of(&[Themed::Input]));

                    f.render_widget(
                        answer_value,
                        Rect::new(
                            chunks[ANSWER_INDEX].x + greeter.prompt_width() as u16,
                            chunks[ANSWER_INDEX].y,
                            get_input_width(greeter, width, &greeter.prompt),
                            1,
                        ),
                    );
                }
            }

            if let Some(message) = message {
                let message = message.alignment(Alignment::Center);

                f.render_widget(message, Rect::new(x, y + height, width, message_height));
            }
        }

        _ => {}
    }

    match greeter.mode {
        Mode::Username => {
            let username_length = greeter.username.get().chars().count();
            let offset = get_cursor_offset(greeter, username_length);

            Ok((
                2 + cursor.x + strings::get("username").chars().count() as u16 + offset as u16,
                USERNAME_INDEX as u16 + cursor.y,
            ))
        }

        Mode::Password => {
            let answer_length = greeter.buffer.chars().count();
            let offset = get_cursor_offset(greeter, answer_length);

            if greeter.asking_for_secret && !greeter.secret_display.shows_input() {
                Ok((
                    1 + cursor.x + greeter.prompt_width() as u16,
                    ANSWER_INDEX as u16 + prompt_padding + cursor.y - 1,
                ))
            } else {
                Ok((
                    1 + cursor.x + greeter.prompt_width() as u16 + offset as u16,
                    ANSWER_INDEX as u16 + prompt_padding + cursor.y - 1,
                ))
            }
        }

        _ => Ok((1, 1)),
    }
}
