# What Rust's compiler catches, and what it doesn't

If you've heard "C and C++ are all you need, just don't write bugs", this
repo is for you. It isn't here to tell you C++ is bad. C++ is fast, mature,
and runs most of the world. It's here to show, with two tiny programs you can
run yourself, what Rust actually changes and what it doesn't:

1. **A bug that C++ compiles and Rust refuses to compile.** Same code, line
   for line. In C++ it silently reads freed memory. In Rust it never builds.
2. **A bug that Rust compiles happily.** Rust's compiler can't save you from
   wrong assumptions. It turns them into a clean crash instead of memory
   corruption, and a fuzzer finds that crash in seconds.

Neither half of the story is "Rust is magic". Together they show where the
line is.

## Setup

You need:

- [Rust](https://rustup.rs/) (stable), plus the nightly toolchain for fuzzing:
  `rustup toolchain install nightly`
- [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz): `cargo install cargo-fuzz`
- `g++` for the C++ example
- [Task](https://taskfile.dev), a small command runner. Run `task` to list
  everything you can do here.

---

## Part 1: Valid C++, rejected by Rust

### The code

Here's the same program in both languages
([`cpp/dangling.cpp`](cpp/dangling.cpp) and [`rust/dangling.rs`](rust/dangling.rs)):

```text
// C++                                     // Rust
std::vector<int> values = {10, 20, 30};    let mut values = vec![10, 20, 30];
int& first = values[0];                    let first = &values[0];
values.push_back(40);                      values.push(40);
std::cout << first << '\n';                println!("{first}");
```

Make a list, take a reference to the first element, add an element, print the
reference. It looks harmless. Most people expect it to print `10`. (This example is
oversimplified, but imagine you're working on a game engine where you want to inspect
one enemy and make it spawn more, and suddenly it becomes a real problem.)

### What goes wrong

A vector stores its elements in one block of heap memory with a fixed
capacity. Here the capacity is 3 and the vector already holds 3 elements, so
adding a fourth means the vector has to:

1. allocate a new, bigger block,
2. copy the elements over,
3. **free the old block.**

`first` still points into the old block. Reading it afterwards reads memory
that has been handed back to the allocator. In C++ that is *undefined
behavior* (UB): the language makes no promises at all about what happens.

### What C++ does

```sh
task cpp
```

Or [try it in your browser on cpp.sh](https://cpp.sh/?source=%2F%2F+Example+1%2C+C%2B%2B+side%3A+compiles+cleanly%2C+but+push_back+reallocates+and%0A%2F%2F+%60first%60+dangles.+Build+with+-fsanitize%3Daddress+to+see+it.%0A%23include+%3Ciostream%3E%0A%23include+%3Cvector%3E%0A%0Aint+main%28%29+%7B%0A++++std%3A%3Avector%3Cint%3E+values+%3D+%7B10%2C+20%2C+30%7D%3B%0A++++int%26+first+%3D+values%5B0%5D%3B%0A++++values.push_back%2840%29%3B%0A++++std%3A%3Acout+%3C%3C+first+%3C%3C+%27%5Cn%27%3B%0A%7D%0A)
and press **Run**.

`g++ -Wall -Wextra` compiles it without a single warning. Running it prints
some number that isn't `10`, like `-1719258260`, and exits normally. On
some machines the number changes every run, on others (cpp.sh, for example)
it's the same wrong number every time. Either way, nothing tells you
anything went wrong. No crash, no error. That's the dangerous part: UB often *looks*
like it works, right up until it doesn't, or until someone exploits it.

The C++ tooling *can* find it, if you ask. Build with AddressSanitizer:

```sh
task cpp:asan
```

```text
ERROR: AddressSanitizer: heap-use-after-free on address 0x502000000010
READ of size 4 at 0x502000000010 thread T0
```

Sanitizers are great, but they only catch bugs on code paths you actually
run while testing, and you have to remember to turn them on. (And then find
the offending line in the code.)

### What Rust does

```sh
task rust
```

Or [try it in your browser on the Rust Playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=66a06e8db844cbfd119fc47859921583)
and press **Run**.

```text
error[E0502]: cannot borrow `values` as mutable because it is also borrowed as immutable
 --> src/main.rs:4:5
  |
3 |     let first = &values[0];
  |                  ------ immutable borrow occurs here
4 |     values.push(40);
  |     ^^^^^^^^^^^^^^^ mutable borrow occurs here
5 |     println!("{first}");
  |                ----- immutable borrow later used here

For more information about this error, try `rustc --explain E0502`.
```

The program never gets built. Rust has one central rule for references, the
*borrow rule*:

> At any moment you can have **either** any number of shared references
> (`&T`, read-only) **or** exactly one mutable reference (`&mut T`), not both.

`first` is a shared reference into `values`. `values.push(40)` needs a mutable
reference, because it may reallocate. Since `first` is still used later, both
would exist at the same time, so the compiler says no. It doesn't need to know
that `push` reallocates. The rule alone rules out the whole class of bug.

### How to fix it in Rust

The compiler is telling you to decide what you actually want. Usually it's
the *value*, not a reference to a slot that might move
([`examples/first.rs`](examples/first.rs)):

```rust
let mut values = vec![10, 20, 30];
let first = values[0]; // copy the i32 out; no reference is kept
values.push(40);
println!("{first}"); // prints 10
```

```sh
task rust:fixed
```

The same fix works in C++ (`int first = values[0];`). The difference is that
C++ lets you forget it and Rust doesn't.

`rust/dangling.rs` is also included in [`src/lib.rs`](src/lib.rs) as a
`compile_fail` doctest, so `task test` fails if that code ever compiles.

---

## Part 2: Valid Rust, crashes under fuzzing

The borrow checker is about memory. It doesn't know what your code is
*supposed* to do. Here's a bug it can't see.

### The code

A function that shortens a string to a 5-character preview
([`src/lib.rs`](src/lib.rs)):

```rust
pub fn preview(input: &str) -> &str {
    if input.len() > 5 { &input[..5] } else { input }
}
```

`preview("Hello world")` returns `"Hello"`. Tests pass. Ship it?

### What goes wrong

Rust strings are UTF-8, and `len()` counts **bytes**, not characters. ASCII
letters take 1 byte each, but other characters take 2 to 4:

```text
"abcd😀"
 a  b  c  d  😀
[0][1][2][3][4 5 6 7]   <- byte offsets
```

`&input[..5]` cuts at byte 5, which is in the middle of the emoji. The result
wouldn't be valid UTF-8, and Rust guarantees every `&str` is valid UTF-8, so
instead of returning garbage it stops the program:

```text
end byte index 5 is not a char boundary; it is inside '😀' (bytes 4..8 of string)
```

This is a *panic*. That's still a bug: in a web server, one weird username
could take a request handler down. But it's a very different kind of failure
from Part 1:

- It's **defined behavior**. The same input crashes the same way every time.
- Nothing reads or writes memory it shouldn't. There's nothing to exploit
  beyond the crash itself.
- You get a clear message pointing at the exact line.

### How a fuzzer finds it

You probably wouldn't think to test with emoji. A *fuzzer* doesn't need to
think. It generates millions of inputs, watches which code paths they reach,
and keeps mutating the interesting ones until something crashes. The fuzz
target is three lines
([`fuzz/fuzz_targets/preview.rs`](fuzz/fuzz_targets/preview.rs)):

```rust
libfuzzer_sys::fuzz_target!(|input: String| {
    let _ = testbed::preview(&input);
});
```

```sh
task fuzz
```

It usually finds a crash within seconds and saves the input under
`fuzz/artifacts/preview/`. On one run it found a 3-byte Chinese character,
not an emoji. Same bug, an input nobody would have written by hand.

### How to fix it

Count characters instead of bytes. `char_indices()` yields each character
with the byte offset where it starts, so the offset of the 6th character is
exactly where the 5-character prefix ends:

```rust
pub fn preview_fixed(input: &str) -> &str {
    let end = input.char_indices().nth(5).map_or(input.len(), |(i, _)| i);
    &input[..end]
}
```

```sh
task fuzz:smoke   # fuzzes preview_fixed for 30 s; FUZZ_TIME=120 for longer
```

No crash, after millions of inputs.

Two more things worth knowing:

- `input.get(..5)` is the non-panicking version of `&input[..5]`. It returns
  `None` instead of crashing when the cut isn't on a character boundary.
- "Character" here means a Unicode *scalar value*. Some things that look like
  one character on screen, such as 👍🏽 or some accented letters, are made of
  several. If you need those, look at the
  [`unicode-segmentation`](https://crates.io/crates/unicode-segmentation)
  crate.

---

## The takeaway

|                                | Part 1 in C++           | Part 2 in Rust          |
|--------------------------------|-------------------------|-------------------------|
| Compiles?                      | Yes, no warnings        | Yes                     |
| Memory safe?                   | No                      | Yes                     |
| How it fails                   | Undefined behavior      | Panic with a message    |
| Compiler catches it?           | No                      | No                      |
| Fuzzer catches it?             | With sanitizers enabled | Yes, out of the box     |
| Exploitable memory corruption? | Possible                | Not from this bug       |

Rust moves a big class of bugs, the "this memory is no longer yours" kind,
from runtime to compile time. It doesn't make your logic correct. Things
that are still worth fuzzing in Rust:

- string slicing by byte index (Part 2)
- indexing with `[]`, which panics when out of bounds (`.get()` doesn't)
- integer overflow, which panics in debug builds and wraps in release
- every `.unwrap()` and `.expect()`, which panic on `None` or `Err`

And one honest caveat: Rust has an `unsafe` keyword that switches some of
these checks off, for things like talking to C libraries. Code in `unsafe`
blocks can have exactly the bugs from Part 1. The point is that you can find
every one of those blocks with a text search (for "unsafe"). An interesting
side effect is that due to the keyword name itself it suddenly feels wrong
to write code the way it was in other low-level languages, and the trend is
to actually instruct the compiler to disallow unsafe code altogether for
a project (`#![forbid(unsafe_code)]`). At the very least it's a good
stop-and-check point for oneself, before writing dodgy code.

## All tasks

| Task | What it does |
|---|---|
| `task cpp` | Build and run the C++ example (prints garbage) |
| `task cpp:asan` | Same with AddressSanitizer; passes if ASan reports the bug |
| `task rust` | Try to compile the Rust version of Part 1 (shows E0502) |
| `task rust:fixed` | Run the fixed Rust version (prints `10`) |
| `task test` | Unit tests plus the `compile_fail` doctest |
| `task fuzz` | Fuzz the buggy `preview` until it crashes |
| `task fuzz:smoke` | Fuzz `preview_fixed` for `FUZZ_TIME` seconds (default 30) |
| `task ci` | Format check, clippy, tests and the ASan check |
| `task build` | Build everything, including the fuzz targets |
| `task clean` / `task fuzz:clean` | Remove build output / fuzzer corpus and crashes |

## License

[EUPL-1.2](LICENSE)
