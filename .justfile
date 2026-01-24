build:
    cargo build --release

install:
    just build
    sudo cp ./target/release/cm /usr/bin/cm
