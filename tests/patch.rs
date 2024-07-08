use unicode_id_start::is_id_continue_unicode;

#[test]
fn legacy_katakana_middle_dot_patch() {
    // U+30FB KATAKANA MIDDLE DOT
    // https://util.unicode.org/UnicodeJsps/character.jsp?a=30FB
    assert!(!is_id_continue_unicode('・'));
    // U+FF65 HALFWIDTH KATAKANA MIDDLE DOT
    // https://util.unicode.org/UnicodeJsps/character.jsp?a=FF65
    assert!(!is_id_continue_unicode('･'));
}
