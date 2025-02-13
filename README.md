# Very much a WIP

## Building

You need to first build the `ctranslate2` C++ library (see [[script/build-ctranslate2.sh]]) and install it via cmake so that it can be found by the `ctranslate2-sys` crate build process.

You will also need the [Intel Compiler](https://www.intel.com/content/www/us/en/developer/tools/oneapi/dpc-compiler.html) and [Intel MKL library](https://www.intel.com/content/www/us/en/developer/tools/oneapi/onemkl.html)

Afterwards, you can build with `cargo build` but you may need to set the cmake prefix path (especially if you're installing in a nonstandard path, see the above script again):

```bash
CMAKE_PREFIX_PATH=/c/ctranslate2/install cargo build
```