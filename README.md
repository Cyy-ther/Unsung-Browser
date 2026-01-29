# Unsung Browser

Unsung Browser — a lightweight, privacy-first web browser written in Rust and designed to run locally on the user's system. It's built to be minimal, configurable, and easy to run from source or a local binary.

> NOTE: This project is still in development and no offical release has been made please feel free to clone it if you want to but note there is some errors in the code i need to fix 

## Features

- Rust-native codebase for performance and safety
- Runs locally on the user's machine — no cloud service required
- Minimal UI with focus on privacy and local control
- Configurable profile and data directory
- Designed for cross-platform builds (Linux, macOS, Windows) — platform support depends on chosen rendering backend
- Developer-friendly: cargo-based build and run workflow

## Quick Start (for developers)

Prerequisites
- Rust toolchain (stable) and Cargo. Install via rustup:
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update stable
  ```
- Platform-specific system libraries (see "Platform-specific deps" below).

Build (debug)
```sh
git clone https://github.com/Cyy-ther/Unsung-Browser.git
cd Unsung-Browser
cargo build
```

Build (release)
```sh
cargo build --release
```

Run
```sh
# From source (debug)
cargo run

# Or run the release binary
./target/release/unsung-browser
# On Windows replace with: target\release\unsung-browser.exe
