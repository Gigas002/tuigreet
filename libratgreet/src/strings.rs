pub fn get(id: &str) -> String {
    match id {
        "failed" => "Authentication failed, please try again.".into(),
        "command_missing" => "No command configured".into(),
        "command_exited" => "Command exited with".into(),
        "command_failed" => "Command failed".into(),
        other => other.to_string(),
    }
}
