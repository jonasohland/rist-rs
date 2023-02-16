#! /bin/bash

set -o pipefail
set -e

function bold() {
    echo -e "\033[1m${1}\033[0m"
}

PROJECT_DIR="$(cd "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"

function install_git_hook_wrapper() {
    echo "Install $(bold "${1}") hook"
    cp -f "${PROJECT_DIR}/scripts/git/resources/${1}.sh" "${PROJECT_DIR}/.git/hooks/${1}"
    chmod +x "${PROJECT_DIR}/.git/hooks/${1}"
}

function install_hook_scripts() {
    mkdir -p "${PROJECT_DIR}/.git/hooks/pre-commit.d"
    for f in "${PROJECT_DIR}/scripts/git/hooks"/*; do
        filename="$(basename "${f}")"
        echo "Install hook script: $(bold "${filename}")"
        cp -f "${f}" "${PROJECT_DIR}/.git/hooks/pre-commit.d/${filename}"
        chmod +x "${PROJECT_DIR}/.git/hooks/pre-commit.d/${filename}"
    done
}

install_git_hook_wrapper "pre-commit"
install_hook_scripts
