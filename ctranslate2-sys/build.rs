use cmake_package::find_package;
use cxx_build;

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
    target.link_directories.iter().for_each(|dir| {
        println!("cargo:warning=\r\x1b[32;1m cargo:rustc-link-search=native={}", dir);
        println!("cargo:rustc-link-search=native={}", dir);
    });
    target.link_options.iter().for_each(|opt| {
        println!("cargo:warning=\r\x1b[32;1m cargo:rustc-link-arg={}", opt);
        println!("cargo:rustc-link-arg={}", opt);
    });
    target.link_libraries.iter().for_each(|lib| {
        if lib.starts_with("-") {
            println!("cargo:warning=\r\x1b[32;1m cargo:rustc-link-arg={}", lib);
            println!("cargo:rustc-link-arg={}", lib);
        } else {
            println!("cargo:warning=\r\x1b[32;1m cargo:rustc-link-lib=dylib={}", link_name(lib));
            println!("cargo:rustc-link-lib={}", link_name(lib));
        }

        if let Some(lib) = link_dir(lib) {
            println!("cargo:warning=\r\x1b[32;1m cargo:rustc-link-search=native={}", lib);
            println!("cargo:rustc-link-search=native={}", lib);
        }
    });

    // Build the C++ bridge for the Rust <-> C++ interop.
    let mut bridge_builder = cxx_build::bridge("lib.rs");
    bridge_builder.flag_if_supported("-std=c++14");
    for include_dir in &target.include_directories {
        bridge_builder.include(include_dir);
    }
    bridge_builder.compile("ctranslate2-sys-bridge");

    // Re-run if the bridge file or any of the included C++ headers change.
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=build.rs");
}

fn link_name(lib: &str) -> String {
    // lib.replace("C:\\", "/c/").replace("\\", "/")
    lib.replace("C:/", "/c/").replace("/bin/", "/lib/").replace(".dll", ".lib")
    // lib.to_string()
}

fn link_dir(_lib: &str) -> Option<&str> {
    None
}