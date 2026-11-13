// Example 1, Rust fix: copy the value out (or index after the push) so no
// borrow outlives the mutation.
fn main() {
    let mut values = vec![10, 20, 30];
    let first = values[0];
    values.push(40);
    println!("{first}");
}
