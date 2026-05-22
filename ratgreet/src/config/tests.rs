use std::path::{Path, PathBuf};

use super::*;

const EXAMPLE_CONFIG: &str = include_str!("../../../examples/config.toml");

#[test]
fn parse_example_config() {
    let cfg = parse(EXAMPLE_CONFIG).unwrap();
    assert_eq!(cfg.keybindings.command, 2);
    assert_eq!(cfg.keybindings.sessions, 3);
    assert_eq!(cfg.keybindings.power, 12);
    assert_eq!(cfg.session.xsession_wrapper, DEFAULT_XSESSION_WRAPPER);
    assert!(!cfg.session.no_xsession_wrapper);
}

#[test]
fn defaults_validate() {
    Config::default().validate().unwrap();
}

#[test]
fn rejects_masked_without_single_mask_char() {
    let err = parse(
        r#"
[secrets]
display = "masked"
mask_char = "**"
"#,
    )
    .unwrap_err();
    assert!(matches!(err, ConfigError::Validation(_)));
}

#[test]
fn rejects_duplicate_keybindings() {
    let err = parse(
        r#"
[keybindings]
command = 2
sessions = 2
power = 12
"#,
    )
    .unwrap_err();
    assert!(matches!(err, ConfigError::Validation(_)));
}

#[test]
fn resolved_paths_prefers_override() {
    let custom = Path::new("/custom/config.toml");
    assert_eq!(
        resolved_paths(Some(custom)),
        vec![PathBuf::from("/custom/config.toml")]
    );
}

#[test]
fn resolved_paths_default_order() {
    let paths = resolved_paths(None);
    assert_eq!(paths[0], system_path());
    assert_eq!(paths[1], user_path());
}

#[test]
fn load_returns_not_found() {
    let result = load(Path::new("/nonexistent/ratgreet/config.toml"));
    assert!(matches!(result, Err(ConfigError::NotFound { .. })));
}

#[test]
fn load_layered_missing_override_uses_defaults() {
    let cfg = load_layered(Some(Path::new("/nonexistent/ratgreet/config.toml")));
    assert_eq!(cfg.keybindings.command, Config::default().keybindings.command);
}

#[test]
fn load_layered_invalid_override_uses_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "not valid toml [[[").unwrap();

    let cfg = load_layered(Some(&path));
    assert_eq!(cfg.keybindings.command, Config::default().keybindings.command);
}

#[test]
fn load_layered_empty_file_uses_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "").unwrap();

    let cfg = load_layered(Some(&path));
    assert_eq!(cfg.keybindings.command, 2);
}

#[test]
fn example_config_file_on_disk() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/config.toml");
    let cfg = load(&path).unwrap();
    assert_eq!(cfg.secrets.display, SecretDisplayMode::default());
}
