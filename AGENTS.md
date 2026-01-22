# Tech Stack
- Rust Edition: 2021

# Rules
- No `.unwrap()`. Use `Result` and `?`.
- Prefer `&str` over `String` for function arguments.
- Use idiomatic Rust (clippy clean).
- Use `cargo fmt -all` to format code.
- Use `cargo check` to verify code is correct.
- Use `cargo test` to verify code is correct.
- Use `cargo clippy --all-targets` to verify code is correct.

# Database
- Postgres database version: 18 syntax and features should be used
- use uuidv7 for uuid generation