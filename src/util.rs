/// Convert a Rust `&str` to a null-terminated UTF-16 wide string.
#[cfg(target_os = "windows")]
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copy a Rust `&str` into a fixed-size `[u16; N]` buffer (null-terminated).
#[cfg(target_os = "windows")]
pub fn to_wide_fixed<const N: usize>(s: &str) -> [u16; N] {
    let mut buf = [0u16; N];
    for (i, ch) in s.encode_utf16().enumerate() {
        if i >= N - 1 {
            break;
        }
        buf[i] = ch;
    }
    buf
}
