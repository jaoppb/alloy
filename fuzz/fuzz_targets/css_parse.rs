#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let _ = css::parse_stylesheet(source, css::Origin::Author);
    let _ = css::tokenize(source);
    let _ = css::parse_inline_style(source);
});
