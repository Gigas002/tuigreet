use super::MaskedString;

#[test]
fn get_value_when_unmasked() {
    let masked = MaskedString::from("value".to_string(), None);

    assert_eq!(masked.get(), "value");
}

#[test]
fn get_mask_when_masked() {
    let masked = MaskedString::from("value".to_string(), Some("mask".to_string()));

    assert_eq!(masked.get(), "mask");
}
