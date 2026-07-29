app:
    cargo +nightly run --bin courier --all-features
hub:
    RUST_LOG=info,courier_hub=debug cargo +nightly run --bin courier-hub
sqlx:
    cargo sqlx prepare --workspace
release target platform:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ target }}" in
        app) bin=courier; features=--all-features; libc=gnu ;;
        hub) bin=courier-hub; features=; libc=musl ;;
        *) echo "unknown target '{{ target }}' (app|hub)" >&2; exit 1 ;;
    esac
    case "{{ platform }}" in
        linux)       triple=x86_64-unknown-linux-$libc; builder=cross ;;
        linux-arm64) triple=aarch64-unknown-linux-$libc; builder=cross ;;
        macos)       triple=aarch64-apple-darwin; builder=cargo ;;
        *) echo "unknown platform '{{ platform }}' (linux|linux-arm64|macos)" >&2; exit 1 ;;
    esac
    if [ "$builder" = cargo ]; then rustup target add "$triple"; fi
    SQLX_OFFLINE=true $builder build --release --bin "$bin" --target "$triple" $features
    echo "built target/$triple/release/$bin"
