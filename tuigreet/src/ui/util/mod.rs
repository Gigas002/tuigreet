use ansi_to_tui::IntoText;
use tui::{
    prelude::Rect,
    text::Text,
    widgets::{Paragraph, Wrap},
};

use libtuigreet::{greeter::Mode, Greeter};

pub fn titleize(message: &str) -> String {
    format!(" {message} ")
}

pub fn buttonize(message: &str) -> String {
    format!(" {message}")
}

// Determinew whether the cursor should be shown or hidden from the current
// mode and configuration. Usually, we will show the cursor only when expecting
// text entries from the user.
pub fn should_hide_cursor(greeter: &Greeter) -> bool {
    greeter.working
        || greeter.done
        || (greeter.user_menu
            && greeter.mode == Mode::Username
            && greeter.username.value.is_empty())
        || (greeter.mode == Mode::Password && greeter.prompt.is_none())
        || greeter.mode == Mode::Users
        || greeter.mode == Mode::Sessions
        || greeter.mode == Mode::Power
        || greeter.mode == Mode::Processing
        || greeter.mode == Mode::Action
}

// Computes the height of the main window where we display content, depending on
// the mode and spacing configuration.
//
// +------------------------+
// |                        | <- container padding
// |        Greeting        | <- greeting height
// |                        | <- auto-padding if greeting
// | Username:              | <- username
// | Password:              | <- password if prompt == Some(_)
// |                        | <- container padding
// +------------------------+
pub fn get_height(greeter: &Greeter) -> u16 {
    let (_, greeting_height) = get_greeting_height(greeter, 1, 0);
    let container_padding = greeter.container_padding();
    let prompt_padding = greeter.prompt_padding();

    let initial = match greeter.mode {
        Mode::Username | Mode::Action | Mode::Command => (2 * container_padding) + 1,
        Mode::Password => match greeter.prompt {
            Some(_) => (2 * container_padding) + prompt_padding + 2,
            None => (2 * container_padding) + 1,
        },
        Mode::Users | Mode::Sessions | Mode::Power | Mode::Processing => 2 * container_padding,
    };

    match greeter.mode {
        Mode::Command | Mode::Sessions | Mode::Power | Mode::Processing => initial,
        _ => initial + greeting_height,
    }
}

// Get the coordinates and size of the main window area, from the terminal size,
// and the content we need to display.
pub fn get_rect_bounds(greeter: &Greeter, area: Rect, items: usize) -> (u16, u16, u16, u16) {
    let width = greeter.width();
    let height: u16 = get_height(greeter) + items as u16;

    let x = if width < area.width {
        (area.width - width) / 2
    } else {
        0
    };
    let y = if height < area.height {
        (area.height - height) / 2
    } else {
        0
    };

    let (x, width) = if (x + width) >= area.width {
        (0, area.width)
    } else {
        (x, width)
    };
    let (y, height) = if (y + height) >= area.height {
        (0, area.height)
    } else {
        (y, height)
    };

    (x, y, width, height)
}

// Computes the size of a text entry, from the container width and, if
// applicable, the prompt length.
pub fn get_input_width(greeter: &Greeter, width: u16, label: &Option<String>) -> u16 {
    let width = std::cmp::min(greeter.width(), width);

    let label_width = match label {
        None => 0,
        Some(label) => label.chars().count(),
    };

    width - label_width as u16 - 4 - 1
}

pub fn get_cursor_offset(greeter: &mut Greeter, length: usize) -> i16 {
    let mut offset = length as i16 + greeter.cursor_offset;

    if offset < 0 {
        offset = 0;
        greeter.cursor_offset = -(length as i16);
    }

    if offset > length as i16 {
        offset = length as i16;
        greeter.cursor_offset = 0;
    }

    offset
}

pub fn get_greeting_height(
    greeter: &Greeter,
    padding: u16,
    fallback: u16,
) -> (Option<Paragraph<'_>>, u16) {
    if let Some(greeting) = &greeter.greeting {
        let width = greeter.width();

        let text = match greeting.clone().trim().into_text() {
            Ok(text) => text,
            Err(_) => Text::raw(greeting),
        };

        let paragraph = Paragraph::new(text.clone()).wrap(Wrap { trim: false });
        let height = paragraph.line_count(width - (2 * padding)) + 1;

        (Some(paragraph), height as u16)
    } else {
        (None, fallback)
    }
}

pub fn get_message_height(
    greeter: &Greeter,
    padding: u16,
    fallback: u16,
) -> (Option<Paragraph<'_>>, u16) {
    if let Some(message) = &greeter.message {
        let width = greeter.width();
        let paragraph = Paragraph::new(message.trim_end()).wrap(Wrap { trim: true });
        let height = paragraph.line_count(width - 4);

        (Some(paragraph), height as u16 + padding)
    } else {
        (None, fallback)
    }
}

#[cfg(test)]
mod tests;
