// Example 1, Rust side: rejected with E0502. Not a Cargo target on purpose;
// `src/lib.rs` includes it as a compile_fail doctest.
fn main() {
    let mut values = vec![10, 20, 30];
    let first = &values[0];
    values.push(40);
    println!("{first}");
}
