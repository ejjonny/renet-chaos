#!/bin/zsh

set -euxo pipefail
(trap 'kill 0' SIGINT; cargo run -p client --release & cargo run -p screen --release & cargo run -p server --release)
wait
