#!/usr/bin/env -S just --justfile

test:
    cargo test --no-fail-fast --workspace --all-features --all-targets

hack:
    cargo install cargo-hack
    cargo hack --feature-powerset check

audit:
    cargo install cargo-audit
    cargo audit

clippy:
    cargo clippy --release --all-targets

clean:
    cargo clean
