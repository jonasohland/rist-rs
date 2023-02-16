#! /bin/bash

function bold() {
    echo -e "\033[1m${1}\033[0m"
}

if ! cargo clippy -- -Dwarnings 2> /dev/null; then
    echo "Clippy has suggestions, run $(bold "cargo clippy") and fix your code"
    exit 1
fi
