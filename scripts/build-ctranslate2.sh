#!/bin/bash
set -e

# Variables
REPO_URL="https://github.com/OpenNMT/CTranslate2.git"
# Modify the installation prefix as needed; for a Windows path, use forward slashes.
# Set default install prefix based on OS
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    DEFAULT_PREFIX="/c/ctranslate2/install"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    DEFAULT_PREFIX="/usr/local"
else
    DEFAULT_PREFIX="/usr"
fi
INSTALL_PREFIX="${CTRANSLATE2_INSTALL_PREFIX:-$DEFAULT_PREFIX}"

# If the CTranslate2 repository doesn't exist, clone it; otherwise, update it.
if [ ! -d "CTranslate2" ]; then
    echo "Cloning CTranslate2 repository from ${REPO_URL}..."
    git clone --recursive ${REPO_URL}
else
    echo "CTranslate2 repository already exists. Updating repository..."
    cd CTranslate2
    git pull
    git submodule update --init --recursive
    cd ..
fi

# Create and enter the build directory
mkdir -p CTranslate2/build
cd CTranslate2/build

# Configure the project with CMake.
# Since we're using the default generator on Windows, this will typically generate a Visual Studio solution or
# use Ninja if it's available. The CMAKE_INSTALL_PREFIX ensures that the package config files get installed correctly.
echo "Configuring CTranslate2 with CMAKE_INSTALL_PREFIX=${INSTALL_PREFIX}..."
cmake -DCMAKE_INSTALL_PREFIX="${INSTALL_PREFIX}" ..

# Build and install the project using the Release configuration.
echo "Building and installing CTranslate2 (Release configuration)..."
cmake --build . --config Release --target INSTALL

echo "CTranslate2 has been successfully built and installed to ${INSTALL_PREFIX}"
