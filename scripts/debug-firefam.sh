#!/bin/bash

# Set "chatgpt.cliExecutable": "/Users/<USERNAME>/code/firefam/scripts/debug-firefam.sh" in VSCode settings to always get the 
# latest firefam-rs binary when debugging Firefam Extension.


set -euo pipefail

FIREFAM_RS_DIR=$(realpath "$(dirname "$0")/../firefam-rs")
(cd "$FIREFAM_RS_DIR" && cargo run --quiet --bin firefam -- "$@")