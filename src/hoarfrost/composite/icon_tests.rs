//! Tests for the shared composite icon value.

use super::Icon;

#[test]
fn test_icon_constructors_preserve_the_glyph() {
    let from_new = Icon::new("⌂");
    let from_str = Icon::from("⌂");
    let from_string = Icon::from(String::from("⌂"));

    assert_eq!(&*from_new, "⌂");
    assert_eq!(from_new, from_str);
    assert_eq!(from_str, from_string);
}
