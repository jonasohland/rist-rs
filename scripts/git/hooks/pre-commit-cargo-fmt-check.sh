#! /bin/bash

function bold() {
    echo -e "\033[1m${1}\033[0m"
}

if ! cargo fmt --check > /dev/null; then
    echo "There are some code style issues, run $(bold "cargo fmt") first"
    exit 1
fi