#pragma once

#include <memory>
#include <vector>
#include <string>
#include "ctranslate2/models/whisper.h"
#include "ctranslate2/storage_view.h"

namespace ctranslate2 {
namespace models {

// Simple factory function for a Whisper object:
inline std::unique_ptr<Whisper> new_whisper(const std::string& model_dir, int /*device*/) {
  // Adjust if you need GPU / device-specific initialization:
  return std::make_unique<Whisper>(model_dir);
}

// Shim that gathers the futures from detect_language() into a single vector:
// inline std::vector<std::pair<std::string, float>>
// detect_language_shim(const Whisper& whisper, const ctranslate2::StorageView& features)
// {
//   // Original call returns a vector<std::future<std::vector<std::pair<std::string,float>>>>.
//   auto futures = whisper.detect_language(features);

//   std::vector<std::pair<std::string, float>> all_results;
//   for (auto &f : futures) {
//     auto partial = f.get(); // each future yields a vector<std::pair<std::string,float>>
//     all_results.insert(all_results.end(), partial.begin(), partial.end());
//   }
//   return all_results;
// }

} // namespace models
} // namespace ctranslate2 