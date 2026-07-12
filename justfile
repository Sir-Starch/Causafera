default:
    @echo "Available commands:"
    @echo "  just ci         - Run CI checks"
    @echo "  just build      - Build workspace"
    @echo "  just test       - Run tests"
    @echo "  just fmt        - Format code"
    @echo "  just doctor     - Run doctor"

 ci:
    cargo xtask ci

build:
    cargo build --workspace --all-features

test:
    cargo test --workspace --all-features

fmt:
    cargo fmt --all

doctor:
    cargo run --bin ontopolis -- doctor
