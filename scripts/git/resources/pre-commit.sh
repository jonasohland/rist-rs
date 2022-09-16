#! /bin/bash

set -e

HOOKS_DIR="$(cd "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/pre-commit.d"

function bold() {
    echo -e "\033[1m${1}\033[0m"
}

for hook_file in "${HOOKS_DIR}/pre-commit"*; do
    echo "Running hook: $(bold "$(basename "${hook_file}")")"
    "${hook_file}"
done