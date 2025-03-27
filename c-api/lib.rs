#[unsafe(no_mangle)]
pub extern "C" fn is_xid_start(c: u32) -> bool {
    if let Some(char) = char::from_u32(c) {
        return unicode_ident::is_xid_start(char);
    };

    false
}

#[unsafe(no_mangle)]
pub extern "C" fn is_xid_continue(c: u32) -> bool {
    if let Some(char) = char::from_u32(c) {
        return unicode_ident::is_xid_continue(char);
    };

    false
}
