use std::path::PathBuf;

use crate::{
    greeter::{Greeter, SecretDisplay},
    settings::{CliOverrides, Settings},
    ui::sessions::SessionSource,
};

#[test]
fn test_prompt_width() {
    let mut greeter = Greeter::default();
    greeter.prompt = None;

    assert_eq!(greeter.prompt_width(), 0);

    greeter.prompt = Some("Hello:".into());

    assert_eq!(greeter.prompt_width(), 6);
}

#[test]
fn test_set_prompt() {
    let mut greeter = Greeter::default();

    greeter.set_prompt("Hello:");

    assert_eq!(greeter.prompt, Some("Hello: ".into()));

    greeter.set_prompt("Hello World: ");

    assert_eq!(greeter.prompt, Some("Hello World: ".into()));

    greeter.remove_prompt();

    assert_eq!(greeter.prompt, None);
}

#[tokio::test]
async fn apply_settings_from_example_config() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/config.toml");
    let settings = Settings::load(&CliOverrides {
        config: Some(path),
        ..CliOverrides::default()
    })
    .unwrap();

    let mut greeter = Greeter::default();
    greeter.apply_settings(&settings);

    assert_eq!(greeter.width, 80);
    assert_eq!(greeter.container_padding, 2);
    assert_eq!(greeter.prompt_padding, 1);
    assert!(!greeter.user_menu);
    assert!(matches!(
        greeter.xsession_wrapper.as_deref(),
        Some("startx /usr/bin/env")
    ));
}

#[tokio::test]
async fn apply_settings_session_cmd_and_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
[session]
cmd = "uname"
env = ["A=B", "C=D=E"]

[secrets]
mask = true
mask_char = "."

[ui]
window_padding = 1
container_padding = 12
prompt_padding = 0
"#,
    )
    .unwrap();

    let settings = Settings::load(&CliOverrides {
        config: Some(config_path),
        ..CliOverrides::default()
    })
    .unwrap();

    let mut greeter = Greeter::default();
    greeter.apply_settings(&settings);

    assert!(
        matches!(&greeter.session_source, SessionSource::DefaultCommand(cmd, Some(env)) if cmd == "uname" && env.len() == 2)
    );
    if let SessionSource::DefaultCommand(_, Some(env)) = &greeter.session_source {
        assert_eq!(env[0], "A=B");
        assert_eq!(env[1], "C=D=E");
    }

    assert!(matches!(&greeter.secret_display, SecretDisplay::Character(c) if c == "."));
    assert_eq!(greeter.window_padding, 1);
    assert_eq!(greeter.container_padding, 13);
    assert_eq!(greeter.prompt_padding, 0);
}

#[tokio::test]
async fn apply_settings_no_xsession_wrapper() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
[session]
no_xsession_wrapper = true
"#,
    )
    .unwrap();

    let settings = Settings::load(&CliOverrides {
        config: Some(config_path),
        ..CliOverrides::default()
    })
    .unwrap();

    let mut greeter = Greeter::default();
    greeter.apply_settings(&settings);

    assert!(greeter.xsession_wrapper.is_none());
}
