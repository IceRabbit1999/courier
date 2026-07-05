fmt:
    cargo +nightly fmt --all
build:
    cargo +nightly build --all-features
run:
    cargo +nightly run --bin courier --all-features
sqlx:
    cargo sqlx prepare --workspace
