# https://github.com/casey/just

fmt:
    cargo fmt --all

check:
    cargo check
    cargo clippy -- -D warnings

test:
    cargo test --all-features -- --test-threads=1

dev: fmt check test

install:
    cargo install --features binary --path .
