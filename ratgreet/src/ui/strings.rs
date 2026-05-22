//! English UI strings (formerly Fluent locales).

pub const DATE_FORMAT: &str = "%a, %d %h %Y - %H:%M";

pub fn get(id: &str) -> String {
    match id {
        "title_command" => "Change session command".into(),
        "title_power" => "Power options".into(),
        "title_session" => "Change session".into(),
        "action_reset" => "Reset".into(),
        "action_command" => "Change command".into(),
        "action_session" => "Choose session".into(),
        "action_power" => "Power".into(),
        "date" => DATE_FORMAT.into(),
        "username" => "Username:".into(),
        "wait" => "Please wait...".into(),
        "failed" => "Authentication failed, please try again.".into(),
        "new_command" => "New command:".into(),
        "shutdown" => "Shut down".into(),
        "reboot" => "Reboot".into(),
        "command_missing" => "No command configured".into(),
        "command_exited" => "Command exited with".into(),
        "command_failed" => "Command failed".into(),
        "status_command" => "CMD".into(),
        "status_session" => "SESS".into(),
        "status_caps" => "CAPS LOCK".into(),
        other => other.to_string(),
    }
}

pub fn title_authenticate(hostname: &str) -> String {
    format!("Authenticate into {hostname}")
}
