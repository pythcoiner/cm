build:
    cargo build --release

install:
    just build
    sudo cp ./target/release/cm /usr/bin/cm

# Refresh `.claude/commands/` in every sibling cm-initialized project by
# running `cm init --force` in each directory under `../` that has a `.cm/`.
# Run `just install` first so `cm` on PATH carries the latest embedded skills.
init-all:
    #!/usr/bin/env bash
    set -euo pipefail
    for dir in ../*/; do
        [ -d "$dir/.cm" ] || continue
        echo "==> $dir"
        (cd "$dir" && cm init --force)
    done
