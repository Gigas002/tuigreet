use std::error::Error;

use libtuigreet::{model::menu::MenuItem, Greeter};
use tui::{
    prelude::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::ui::{
    common::style::{Theme, Themed},
    util::{get_rect_bounds, titleize},
    Frame,
};

pub trait DrawMenu {
    fn draw(
        &self,
        greeter: &Greeter,
        theme: &Theme,
        f: &mut Frame,
    ) -> Result<(u16, u16), Box<dyn Error>>;
}

impl<T> DrawMenu for libtuigreet::model::menu::Menu<T>
where
    T: MenuItem,
{
    fn draw(
        &self,
        greeter: &Greeter,
        theme: &Theme,
        f: &mut Frame,
    ) -> Result<(u16, u16), Box<dyn Error>> {
        let size = f.size();
        let (x, y, width, height) = get_rect_bounds(greeter, size, self.options.len());

        let container = Rect::new(x, y, width, height);

        let title = Span::from(titleize(&self.title));
        let block = Block::default()
            .title(title)
            .title_style(theme.of(&[Themed::Title]))
            .style(theme.of(&[Themed::Container]))
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(theme.of(&[Themed::Border]));

        for (index, option) in self.options.iter().enumerate() {
            let name = option.format();
            let name = format!("{:1$}", name, greeter.width() as usize - 4);

            let frame = Rect::new(x + 2, y + 2 + index as u16, width - 4, 1);
            let option_text = menu_option_span(self.selected == index, name);
            let option = Paragraph::new(option_text);

            f.render_widget(option, frame);
        }

        f.render_widget(block, container);

        Ok((1, 1))
    }
}

fn menu_option_span<'g, S>(selected: bool, name: S) -> Span<'g>
where
    S: Into<String>,
{
    if selected {
        Span::styled(
            name.into(),
            Style::default().add_modifier(Modifier::REVERSED),
        )
    } else {
        Span::from(name.into())
    }
}
