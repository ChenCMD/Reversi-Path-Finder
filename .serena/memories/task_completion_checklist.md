# Task completion checklist
- Ensure code is formatted (`cargo fmt`).
- Run lints (`cargo clippy --all-targets --all-features`) and address warnings when feasible.
- Run tests (`cargo test`); add/adjust tests if behavior changes.
- If solver behavior is touched, consider a sanity run (`cargo run` or `cargo run -- --help` if args added) and note any solver binary requirements (Yices2 + Kissat).
- Summarize changes and any manual steps (e.g., external solver setup) for the user.
