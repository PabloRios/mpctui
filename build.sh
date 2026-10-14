#!/usr/bin/env bash
# Compila mpctui. Uso: ./build.sh [debug|release|install]   (por defecto: release)
# install: compila en release y lo instala en ~/.cargo/bin (reemplaza el anterior).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"
export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo no está instalado (https://rustup.rs)" >&2
    exit 1
fi

profile="${1:-release}"
case "$profile" in
    release) cargo build --release ;;
    debug)   cargo build ;;
    install)
        cargo install --path . --force
        echo "Instalado: $HOME/.cargo/bin/mpctui"
        exit 0
        ;;
    *)
        echo "uso: $0 [debug|release|install]" >&2
        exit 1
        ;;
esac

if [ "$profile" = "release" ]; then
    echo "Listo: $(pwd)/target/release/mpctui"
else
    echo "Listo: $(pwd)/target/debug/mpctui"
fi
