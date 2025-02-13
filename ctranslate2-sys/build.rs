use cmake_package::find_package;

fn main() {
    // Try to locate the installed CTranslate2 package.
    // Adjust the package name and required version as needed.
    let package = find_package("CTranslate2")
        .version("2.0") // Specify a minimum version if desired.
        .find()
        .expect("CTranslate2 package not found. Please install it on your system.");

    // Query the package for the CTranslate2 target.
    // The target name must match exactly what the CMake package defines.
    let target = package
        .target("CTranslate2::ctranslate2")
        .expect("CTranslate2 target not found in the package.");

    // For debugging purposes, print out the include directories.
    println!("Include directories: {:?}", target.include_directories);

    // Instruct Cargo to link the final binary against this target.
    // This prints the necessary cargo:rustc-link-* directives.
    target.link();

    // Re-run the build script if this file changes.
    // println!("cargo:rerun-if-changed=build.rs");
}