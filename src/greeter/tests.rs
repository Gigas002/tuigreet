use crate::{
    greeter::{Greeter, SecretDisplay},
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
#[allow(clippy::type_complexity)]
async fn test_command_line_arguments() {
    let table: &[(&[&str], _, Option<fn(&Greeter)>)] = &[
        (&[], true, None),
        (&["--cmd", "hello"], true, None),
        (
            &[
                "--cmd",
                "uname",
                "--env",
                "A=B",
                "--env",
                "C=D=E",
                "--asterisks",
                "--asterisks-char",
                ".",
                "--issue",
                "--time",
                "--prompt-padding",
                "0",
                "--window-padding",
                "1",
                "--container-padding",
                "12",
                "--user-menu",
            ],
            true,
            Some(|greeter| {
                assert!(
                    matches!(&greeter.session_source, SessionSource::DefaultCommand(cmd, Some(env)) if cmd == "uname" && env.len() == 2)
                );

                if let SessionSource::DefaultCommand(_, Some(env)) = &greeter.session_source {
                    assert_eq!(env[0], "A=B");
                    assert_eq!(env[1], "C=D=E");
                }

                assert!(matches!(&greeter.secret_display, SecretDisplay::Character(c) if c == "."));
                assert_eq!(greeter.prompt_padding(), 0);
                assert_eq!(greeter.window_padding(), 1);
                assert_eq!(greeter.container_padding(), 13);
                assert!(greeter.user_menu);
                assert!(matches!(
                    greeter.xsession_wrapper.as_deref(),
                    Some("startx /usr/bin/env")
                ));
            }),
        ),
        (
            &["--xsession-wrapper", "mywrapper.sh"],
            true,
            Some(|greeter| {
                assert!(matches!(
                    greeter.xsession_wrapper.as_deref(),
                    Some("mywrapper.sh")
                ));
            }),
        ),
        (
            &["--no-xsession-wrapper"],
            true,
            Some(|greeter| {
                assert!(greeter.xsession_wrapper.is_none());
            }),
        ),
        (
            &["--remember-session", "--remember-user-session"],
            false,
            None,
        ),
        (&["--asterisk-char", ""], false, None),
        (&["--remember-user-session"], false, None),
        (&["--min-uid", "10000", "--max-uid", "5000"], false, None),
        (&["--issue", "--greeting", "Hello, world!"], false, None),
        (&["--kb-command", "F2", "--kb-sessions", "F2"], false, None),
        (&["--time-format", "%i %"], false, None),
        (&["--cmd", "cmd", "--env"], false, None),
        (&["--cmd", "cmd", "--env", "A"], false, None),
    ];

    for (opts, valid, check) in table {
        let mut greeter = Greeter::default();

        match valid {
            true => {
                assert!(
                    matches!(greeter.parse_options(opts).await, Ok(())),
                    "{:?} cannot be parsed",
                    opts
                );

                if let Some(check) = check {
                    check(&greeter);
                }
            }
            false => assert!(greeter.parse_options(opts).await.is_err()),
        }
    }
}
