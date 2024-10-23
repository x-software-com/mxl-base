#!/usr/bin/env -S just --justfile
#
# To run this script, you must have installed the Just command runner. Execute:
# $ cargo install --locked just

#
# Setup the environment:
#

setup-cargo-hack:
    cargo install --locked cargo-hack

setup-cargo-audit:
    cargo install --locked cargo-audit

setup: setup-cargo-hack setup-cargo-audit
    git config pull.rebase true
    git config branch.autoSetupRebase always
    cargo install --locked typos-cli
    cargo install --locked cocogitto
    cog install-hook --overwrite commit-msg
    @echo "Done"

#
# Recipes for test and linting:
#

hack: setup-cargo-hack
    cargo hack --feature-powerset check

audit: setup-cargo-audit
    cargo audit

clippy:
    cargo clippy --release --all-targets

cargo-fmt:
    cargo fmt --all

#
# Misc recipes:
#

clean:
    cargo clean
