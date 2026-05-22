use std::borrow::Cow;

pub trait MenuItem {
    fn format(&self) -> Cow<'_, str>;
}

#[derive(Default)]
pub struct Menu<T>
where
    T: MenuItem,
{
    pub title: String,
    pub options: Vec<T>,
    pub selected: usize,
}
