// Import the cxx crate types.
use cxx::{CxxString, UniquePtr, CxxVector};

#[cxx::bridge]
mod ffi {
    #[namespace = "ctranslate2"]
    unsafe extern "C++" {
        // Include the CTranslate2 header for the StorageView.
        include!("ctranslate2/storage_view.h");

        // Declare opaque types corresponding to the C++ classes.
        type StorageView;
    }

    #[namespace = "ctranslate2::models"]
    unsafe extern "C++" {
        // Use our new bridging header:
        include!("whisper_bridge.h");

        // The Whisper model is defined in the nested namespace (ctranslate2::models)
        // but here we assume our shim functions "unify" the namespace.
        type Whisper;

        // Creates a new Whisper model instance.
        // The model directory is provided as a C++ string and device is an integer (e.g. 0 for CPU).
        fn new_whisper(model_dir: &CxxString, device: i32) -> UniquePtr<Whisper>;

        // A shim that gathers the results of detect_language into a single vector:
        // fn detect_language_shim(self: &Whisper, features: &StorageView) -> Vec<LangPair>;

        // // Creates a new StorageView from a pointer to an f32 array and a shape.
        // // The shape is provided as a C++ vector of usize.
        // unsafe fn new_storage_view(data: *const f32, shape: &CxxVector<usize>) -> UniquePtr<StorageView>;

        // Calls the detect_language method on a Whisper instance.
        // For simplicity, we assume that for a given input (a StorageView)
        // the function returns a vector (one per example) of LanguageResult.
        // fn detect_language(self: &Whisper, features: &StorageView) -> Vec<LanguageResult>;

        // Calls the generate method on a Whisper instance.
        // It takes the features and a prompt (passed as a C++ vector of i32 tokens)
        // and returns a GenerationResult.
        // fn generate(self: &Whisper, features: &StorageView, prompt_tokens: &CxxVector<i32>) -> GenerationResult;
    }

    // If "LanguageResult" is purely your own invention (i.e. not defined in C++),
    // then *this* can be a shared struct. There's no conflict with existing C++ code.
    // struct LanguageResult {
    //     language: String,
    //     probability: f32,
    // }

    // // This struct will hold each (language, probability) pair
    // // returned by detect_language_shim.
    // struct LangPair {
    //     first: String,
    //     second: f32,
    // }

    // Finally, remove or rename your old GenerationResult if it conflicts 
    // with the real C++ struct. For example, if you need a custom result 
    // that doesn't map to the library's GenerationResult, rename it:
    //
    // struct MyRustGenResult {
    //     sequences_ids: Vec<i32>,
    // }
}

// Define our Rust wrapper around the C++ Whisper opaque type.
pub struct Whisper {
    inner: UniquePtr<ffi::Whisper>,
}

impl Whisper {
    /// Creates a new Whisper model instance.
    ///
    /// # Parameters
    /// - `model_dir`: The model directory. For now, this must be a string literal (or &'static str)
    ///   because `let_cxx_string!` only supports compile-time strings.
    /// - `device`: An integer representing the device (e.g. 0 for CPU).
    pub fn new(model_dir: &'static str, device: i32) -> Self {
        cxx::let_cxx_string!(cxx_model_dir = model_dir);
        let inner = ffi::new_whisper(&cxx_model_dir, device);
        Whisper { inner }

    }
}