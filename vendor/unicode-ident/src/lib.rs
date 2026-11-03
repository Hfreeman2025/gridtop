//! Clean-room compatibility adapter. No unicode-ident code or tables are copied.
//! XID classification is supplied by the MIT/Apache-2.0 unicode-xid crate.
#![no_std]
#![forbid(unsafe_code)]

pub fn is_xid_start(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_start(c)
}

pub fn is_xid_continue(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(c)
}
