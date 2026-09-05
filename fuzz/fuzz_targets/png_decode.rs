#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = graphics::infrastructure::png_decode::decode_png(data);
});
