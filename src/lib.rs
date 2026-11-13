//! Example 1: borrow checker rejects a dangling reference that C++ accepts
//! (see `cpp/dangling.cpp`). `rust/dangling.rs` is included as a
//! `compile_fail` doctest so `cargo test` proves it still fails to compile.
#![doc = concat!("```compile_fail,E0502\n", include_str!("../rust/dangling.rs"), "\n```")]

/// Example 2, buggy: slices by bytes, panics when byte 5 is inside a char.
pub fn preview(input: &str) -> &str {
    if input.len() > 5 { &input[..5] } else { input }
}

/// Example 2, fixed: first five Unicode scalar values.
pub fn preview_fixed(input: &str) -> &str {
    let end = input.char_indices().nth(5).map_or(input.len(), |(i, _)| i);
    &input[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_passes() {
        assert_eq!(preview("Hello world"), "Hello");
    }

    #[test]
    #[should_panic(expected = "is not a char boundary")]
    fn emoji_panics() {
        preview("abcd😀");
    }

    #[test]
    fn fixed_handles_emoji() {
        assert_eq!(preview_fixed("abcd😀xyz"), "abcd😀");
        assert_eq!(preview_fixed("Hello world"), "Hello");
        assert_eq!(preview_fixed("hi"), "hi");
    }
}
