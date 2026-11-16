set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]

dev-fast:
    cargo run --locked -p bongocat-app

dev: dev-fast

dev-smoke:
    cargo run --locked -p bongocat-app -- --run-seconds 5

build:
    cargo build --locked -p bongocat-app --release --features production

test:
    cargo test --locked --workspace

check:
    cargo fmt --all -- --check
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
    cargo test --locked --workspace
    cargo check --locked --workspace --release
