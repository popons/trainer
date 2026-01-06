#!/bin/zsh

export CARGO_TERM_COLOR=always

# Check if running on WSL1 (kernel contains Microsoft/WSL but not WSL2)
KERNEL=$(uname -r)
if echo "$KERNEL" | grep -qE '(Microsoft|WSL)' && echo "$KERNEL" | grep -qv 'WSL2'; then
    POLL_FLAG="--poll"
else
    POLL_FLAG=""
fi

cargo watch -w server -w wasm -w core $POLL_FLAG  -s '
    RUST_LOG=debug cd wasm && \
    wasm-pack build --target web --dev && \
    cd .. && \
    RUST_LOG=debug cargo run --bin wtn -- serve'
