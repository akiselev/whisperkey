# GitHub Actions Workflow for Cross-Platform Builds

This directory contains GitHub Actions workflows for building Whisperkey across multiple platforms.

## Build Workflow (`build.yml`)

The main build workflow creates releases for three platforms:

1. **Windows** - Uses cross-compilation from a Linux container with MinGW-w64
2. **Linux** - Builds natively on a Linux container
3. **macOS** - Builds natively on a macOS runner

### Key Features

- **Compilation Caching** - Uses [sccache](https://github.com/mozilla/sccache) to speed up builds
- **Dependency Caching** - Caches Cargo registry and git repositories between runs
- **Artifacts** - Uploads build artifacts for each platform

### How Caching Works

The workflow uses two types of caching:

1. **GitHub Actions Cache** - Persists the Cargo registry, git repositories, and the sccache cache directory between workflow runs
2. **sccache** - Provides compilation caching during the build process

Both mechanisms significantly speed up builds, especially when dependencies haven't changed.

### Cross-Compilation

Windows builds are cross-compiled using MinGW-w64 toolchain from a Linux container, avoiding the need for MSVC. This approach:

- Provides consistent build environments
- Simplifies Windows builds
- Reduces build times through better caching

### Environment Setup

Each build job:

1. Sets up the required development environment including:

   - Rust toolchain
   - Platform-specific dependencies
   - Intel MKL library
   - ctranslate2 library

2. Configures cross-compilation toolchains where needed

3. Configures environment variables for compilation

### Artifacts

After successful builds, the workflow uploads platform-specific binaries as artifacts that can be downloaded from the GitHub Actions run page.

## Usage

The workflow runs automatically on:

- Pushes to `main` branch
- Pull requests to `main` branch
- Manual trigger via GitHub Actions UI (workflow_dispatch)

## Troubleshooting

If the workflow fails:

1. Check the sccache statistics in the logs to see if caching is working correctly
2. Verify that the dependencies are installing correctly
3. Ensure that the Intel MKL and ctranslate2 libraries are being built and installed correctly
