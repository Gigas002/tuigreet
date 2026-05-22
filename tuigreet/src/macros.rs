#[macro_export]
macro_rules! fl {
    ("title_authenticate", hostname = $hostname:expr) => {{
        $crate::ui::strings::title_authenticate(&$hostname)
    }};

    ($message_id:literal) => {{
        $crate::ui::strings::get($message_id)
    }};
}
