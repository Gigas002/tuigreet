use crate::{
    Greeter,
    model::{
        menu::Menu,
        sessions::{Session, SessionSource, SessionType},
    },
};

#[test]
fn from_path_existing() {
    let mut greeter = Greeter::default();
    greeter.session_source = SessionSource::Session(1);

    greeter.sessions = Menu::<Session> {
        title: "Sessions".into(),
        selected: 1,
        options: vec![
            Session {
                name: "Session1".into(),
                command: "Session1Cmd".into(),
                session_type: SessionType::Wayland,
                path: Some("/Session1Path".into()),
                ..Default::default()
            },
            Session {
                name: "Session2".into(),
                command: "Session2Cmd".into(),
                session_type: SessionType::X11,
                path: Some("/Session2Path".into()),
                ..Default::default()
            },
        ],
    };

    let session = Session::from_path(&greeter, "/Session2Path");

    assert!(session.is_some());
    assert_eq!(session.unwrap().name, "Session2");
    assert_eq!(session.unwrap().session_type, SessionType::X11);
}

#[test]
fn from_path_non_existing() {
    let mut greeter = Greeter::default();
    greeter.session_source = SessionSource::Session(1);

    greeter.sessions = Menu::<Session> {
        title: "Sessions".into(),
        selected: 1,
        options: vec![Session {
            name: "Session1".into(),
            command: "Session1Cmd".into(),
            session_type: SessionType::Wayland,
            path: Some("/Session1Path".into()),
            ..Default::default()
        }],
    };

    let session = Session::from_path(&greeter, "/Session2Path");

    assert!(session.is_none());
}

#[test]
fn no_session() {
    let greeter = Greeter::default();

    assert!(Session::get_selected(&greeter).is_none());
}

#[test]
fn distinct_session() {
    let mut greeter = Greeter::default();
    greeter.session_source = SessionSource::Session(1);

    greeter.sessions = Menu::<Session> {
        title: "Sessions".into(),
        selected: 1,
        options: vec![
            Session {
                name: "Session1".into(),
                command: "Session1Cmd".into(),
                session_type: SessionType::Wayland,
                path: Some("/Session1Path".into()),
                ..Default::default()
            },
            Session {
                name: "Session2".into(),
                command: "Session2Cmd".into(),
                session_type: SessionType::X11,
                path: Some("/Session2Path".into()),
                ..Default::default()
            },
        ],
    };

    let session = Session::get_selected(&greeter);

    assert!(session.is_some());
    assert_eq!(session.unwrap().name, "Session2");
    assert_eq!(session.unwrap().session_type, SessionType::X11);
}

#[test]
fn same_name_session() {
    let mut greeter = Greeter::default();
    greeter.session_source = SessionSource::Session(1);

    greeter.sessions = Menu::<Session> {
        title: "Sessions".into(),
        selected: 1,
        options: vec![
            Session {
                name: "Session".into(),
                command: "Session1Cmd".into(),
                session_type: SessionType::Wayland,
                path: Some("/Session1Path".into()),
                ..Default::default()
            },
            Session {
                name: "Session".into(),
                command: "Session2Cmd".into(),
                session_type: SessionType::X11,
                path: Some("/Session2Path".into()),
                ..Default::default()
            },
        ],
    };

    let session = Session::get_selected(&greeter);

    assert!(session.is_some());
    assert_eq!(session.unwrap().name, "Session");
    assert_eq!(session.unwrap().session_type, SessionType::X11);
    assert_eq!(session.unwrap().command, "Session2Cmd");
}
