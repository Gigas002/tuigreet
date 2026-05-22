use std::path::PathBuf;

use crate::{
    greeter::Greeter,
    ipc::{DefaultCommand, desktop_names_to_xdg},
    model::sessions::{Session, SessionType},
};

use super::wrap_session_command;

#[test]
fn wayland_no_wrapper() {
    let greeter = Greeter::default();

    let session = Session {
        name: "Session1".into(),
        session_type: SessionType::Wayland,
        command: "Session1Cmd".into(),
        path: Some(PathBuf::from("/Session1Path")),
        ..Default::default()
    };

    let default = DefaultCommand(&session.command, None);
    let (command, env) = wrap_session_command(&greeter, Some(&session), &default);

    assert_eq!(command.as_ref(), "Session1Cmd");
    assert_eq!(env, vec!["XDG_SESSION_TYPE=wayland"]);
}

#[test]
fn wayland_wrapper() {
    let mut greeter = Greeter::default();
    greeter.session_wrapper = Some("/wrapper.sh".into());

    let session = Session {
        name: "Session1".into(),
        session_type: SessionType::Wayland,
        command: "Session1Cmd".into(),
        path: Some(PathBuf::from("/Session1Path")),
        ..Default::default()
    };

    let default = DefaultCommand(&session.command, None);
    let (command, env) = wrap_session_command(&greeter, Some(&session), &default);

    assert_eq!(command.as_ref(), "/wrapper.sh Session1Cmd");
    assert_eq!(env, vec!["XDG_SESSION_TYPE=wayland"]);
}

#[test]
fn x11_wrapper() {
    let mut greeter = Greeter::default();
    greeter.xsession_wrapper = Some("startx /usr/bin/env".into());

    let session = Session {
        slug: Some("thede".to_string()),
        name: "Session1".into(),
        session_type: SessionType::X11,
        command: "Session1Cmd".into(),
        path: Some(PathBuf::from("/Session1Path")),
        xdg_desktop_names: Some("one;two;three;".to_string()),
    };

    let default = DefaultCommand(&session.command, None);
    let (command, env) = wrap_session_command(&greeter, Some(&session), &default);

    assert_eq!(command.as_ref(), "startx /usr/bin/env Session1Cmd");
    assert_eq!(
        env,
        vec![
            "XDG_SESSION_DESKTOP=thede",
            "DESKTOP_SESSION=thede",
            "XDG_SESSION_TYPE=x11",
            "XDG_CURRENT_DESKTOP=one:two:three"
        ]
    );
}

#[test]
fn xdg_current_desktop() {
    assert_eq!(
        desktop_names_to_xdg("one;two;three four"),
        "one:two:three four"
    );
    assert_eq!(desktop_names_to_xdg("one;"), "one");
    assert_eq!(desktop_names_to_xdg(""), "");
    assert_eq!(desktop_names_to_xdg(";"), "");
}
