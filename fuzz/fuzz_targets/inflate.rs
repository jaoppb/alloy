#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = network::inflate::inflate(data);
    let _ = network::inflate::zlib_decompress(data);
    let _ = network::inflate::gzip_decompress(data);
});
