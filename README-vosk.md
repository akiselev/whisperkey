# Vosk Speech Recognition for WhisperKey

WhisperKey uses the Vosk speech recognition library for transcribing audio. This document explains how to download and use Vosk models with WhisperKey.

## What is Vosk?

[Vosk](https://alphacephei.com/vosk/) is an open-source speech recognition toolkit that supports 20+ languages and works offline. It offers models of different sizes, trading off accuracy for speed and resource usage.

## Downloading Vosk Models

### Using the Provided Script

We provide a shell script to download Vosk models automatically:

```bash
# Download the default English small model
./scripts/download_vosk_model.sh

# Download a specific language model
./scripts/download_vosk_model.sh --language de --size medium

# Download to a custom location
./scripts/download_vosk_model.sh --output /path/to/models
```

### Manual Download

If you prefer to download models manually:

1. Visit the [Vosk Models page](https://alphacephei.com/vosk/models)
2. Download the appropriate model for your language and requirements
3. Extract the ZIP file to a directory of your choice
4. Configure WhisperKey to use this model directory

## Model Sizes and Performance

Vosk offers models in different sizes:

- **Small models** (~40MB): Fastest, lowest resource usage, but less accurate
- **Medium models** (~1GB): Good balance between speed and accuracy
- **Large models** (~2-4GB): Most accurate, but require more CPU power and memory

For desktop use, medium models offer a good balance between accuracy and performance.

## Supported Languages

Vosk supports 20+ languages including:

- English (US, UK, Indian)
- Spanish
- French
- German
- Russian
- Portuguese
- Chinese
- Italian
- Dutch
- Catalan
- and more...

See the [Vosk Models page](https://alphacephei.com/vosk/models) for the full list.

## Model Path Configuration

WhisperKey searches for Vosk models in these locations:

1. `./model` directory in the current working directory
2. `~/vosk-model` in the user's home directory
3. `/usr/share/vosk-model` (Linux)
4. `C:/Program Files/Vosk/model` (Windows)
5. `~/Library/Application Support/Vosk/model` (macOS)

You can also specify a custom path when launching WhisperKey with the `--model-path` argument.

## Troubleshooting

### Model Not Found

If WhisperKey cannot find your Vosk model:

1. Check that the model is properly extracted and contains files like `conf`, `am`, `graph`, etc.
2. Place the model files in one of the standard locations or specify the path explicitly
3. Check the console output for any error messages related to the model

### Poor Recognition Quality

If speech recognition quality is poor:

1. Try a larger model (medium or large instead of small)
2. Ensure your microphone is properly configured and capturing clear audio
3. Verify you're using the correct model for your language
4. Speak clearly and at a moderate pace

## Advanced Configuration

The Vosk recognizer supports additional configuration options not exposed in the WhisperKey UI:

- Sample rate: WhisperKey uses 16kHz by default, which works well with Vosk models
- Number of alternatives: Currently set to 1
- Word timestamps: Enabled by default

Future versions may expose these settings in the configuration UI.
