use super::Greeter;

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
