cmake_minimum_required(VERSION 3.19)
# Enable find_package debug tracing
set(CMAKE_FIND_DEBUG_MODE ON)

# Call find_package (adjust the package name and REQUIRED flag as needed)
find_package(CTranslate2 REQUIRED)

# Optionally, print a message if found
message("CTranslate2 found: \${CTranslate2_FOUND}")