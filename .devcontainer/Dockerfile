FROM mcr.microsoft.com/devcontainers/rust:1-bullseye

# Install Windows cross-compilation tools
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    gcc-mingw-w64 g++-mingw-w64 \
    binutils-mingw-w64 \
    pkg-config \
    cmake \
    make \
    git \
    curl \
    libssl-dev \
    libudev-dev \
    libgtk-3-dev \
    build-essential \
    ninja-build \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Configure environment variables for cross compilation
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc \
    CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++ \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    WINDRES=x86_64-w64-mingw32-windres

# Create a .cargo/config.toml for the user to help with cross-compilation
RUN mkdir -p /home/vscode/.cargo
COPY <<EOF /home/vscode/.cargo/config.toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-gcc-ar"
EOF

RUN chown -R vscode:vscode /home/vscode/.cargo

# Install sccache for faster builds
ARG SCCACHE_VERSION="0.7.4"
RUN curl -L "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" | tar xz \
    && install -m 755 sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache \
    && rm -rf sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl

WORKDIR /workspaces 