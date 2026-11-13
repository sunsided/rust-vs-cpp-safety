#![no_main]
libfuzzer_sys::fuzz_target!(|input: String| {
    let _ = testbed::preview_fixed(&input);
});
