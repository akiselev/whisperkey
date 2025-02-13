// Import the cxx crate types.
use cxx::{CxxString, UniquePtr, CxxVector};

#[cxx::bridge(namespace = "ctranslate2")]
mod ffi {
    // In the extern "C++" block we include the relevant C++ header files.
    unsafe extern "C++" {
        // Include the CTranslate2 header for the StorageView.
        include!("ctranslate2/storage_view.h");
        // Include the model header for Whisper.
        include!("ctranslate2/models/whisper.h");

        // Declare opaque types corresponding to the C++ classes.
        type StorageView;
        // The Whisper model is defined in the nested namespace (ctranslate2::models)
        // but here we assume our shim functions “unify” the namespace.
        type Whisper;

        // Creates a new Whisper model instance.
        // The model directory is provided as a C++ string and device is an integer (e.g. 0 for CPU).
        fn new_whisper(model_dir: &CxxString, device: i32) -> UniquePtr<Whisper>;

        // Creates a new StorageView from a pointer to an f32 array and a shape.
        // The shape is provided as a C++ vector of usize.
        unsafe fn new_storage_view(data: *const f32, shape: &CxxVector<usize>) -> UniquePtr<StorageView>;

        // Calls the detect_language method on a Whisper instance.
        // For simplicity, we assume that for a given input (a StorageView)
        // the function returns a vector (one per example) of LanguageResult.
        fn detect_language(self: &Whisper, features: &StorageView) -> Vec<LanguageResult>;

        // Calls the generate method on a Whisper instance.
        // It takes the features and a prompt (passed as a C++ vector of i32 tokens)
        // and returns a GenerationResult.
        fn generate(self: &Whisper, features: &StorageView, prompt_tokens: &CxxVector<i32>) -> GenerationResult;
    }

    // Shared structures that are visible to both Rust and C++.
    // LanguageResult corresponds to a (language, probability) pair.
    struct LanguageResult {
        language: String,
        probability: f32,
    }

    // GenerationResult holds the list of generated token IDs.
    struct GenerationResult {
        sequences_ids: Vec<i32>,
    }
}

// Re-export the bridge module types for easier use.
pub use ffi::*;