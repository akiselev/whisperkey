# Whisperkey Development Container

This devcontainer provides a complete development environment for cross-compiling Whisperkey to Windows.

## Features

- Rust toolchain with Windows cross-compilation support
- MinGW-w64 for Windows cross-compilation
- CMake and build essentials
- Sccache for faster builds
- Support for vosk-based speech recognition

## Getting Started

1. Install [Docker](https://www.docker.com/products/docker-desktop) and [Visual Studio Code](https://code.visualstudio.com/)
2. Install the [Remote - Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) extension in VS Code
3. Clone this repository and open it in VS Code
4. When prompted, click "Reopen in Container" or run the command "Remote-Containers: Reopen in Container" from the command palette (F1)
5. Wait for the container to build - this may take some time on first run

## Cross-Compiling to Windows

To cross-compile the project for Windows:

```bash
# Add the Windows target if not already added
rustup target add x86_64-pc-windows-gnu

# Build for Windows
cargo build --target x86_64-pc-windows-gnu
```

The built binaries will be in `target/x86_64-pc-windows-gnu/debug/` or `target/x86_64-pc-windows-gnu/release/` if you build with `--release`.

## Environment Variables

The devcontainer sets up the following environment variables:

- Cross-compilation environment variables for the MinGW toolchain
- Sccache configuration for faster builds

## Troubleshooting

If you encounter issues with cross-compilation:

1. Verify that your `.cargo/config.toml` has the correct settings for cross-compilation:

   ```toml
   [target.x86_64-pc-windows-gnu]
   linker = "x86_64-w64-mingw32-gcc"
   ar = "x86_64-w64-mingw32-gcc-ar"
   ```

2. Check if sccache is working correctly:

   ```bash
   sccache --show-stats
   ```

3. For vosk-related issues, refer to the README-vosk.md file in the main repository.
