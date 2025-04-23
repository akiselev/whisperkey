#!/bin/bash
# Simple script to download Vosk models for WhisperKey

MODEL_SIZES=("small" "medium" "large")
DEFAULT_SIZE="small"
DEFAULT_LANG="en-us"
OUTPUT_DIR="./model"

# Check if curl or wget is available
if command -v curl &> /dev/null; then
    DOWNLOAD_CMD="curl -L"
elif command -v wget &> /dev/null; then
    DOWNLOAD_CMD="wget -O -"
else
    echo "Error: Neither curl nor wget is installed. Please install one of them and try again."
    exit 1
fi

# Display usage information
function show_usage {
    echo "Usage: $0 [options]"
    echo "Download Vosk speech recognition models for WhisperKey"
    echo ""
    echo "Options:"
    echo "  -l, --language LANG   Language code (default: $DEFAULT_LANG)"
    echo "  -s, --size SIZE       Model size: small, medium, large (default: $DEFAULT_SIZE)"
    echo "  -o, --output DIR      Output directory (default: $OUTPUT_DIR)"
    echo "  -h, --help            Show this help message"
    echo ""
    echo "Available language codes: en-us, en-in, fr, de, es, pt, it, nl, ru, etc."
    echo "See https://alphacephei.com/vosk/models for all available models"
    exit 0
}

# Parse command line arguments
LANG="$DEFAULT_LANG"
SIZE="$DEFAULT_SIZE"
OUTPUT="$OUTPUT_DIR"

while [[ $# -gt 0 ]]; do
    case "$1" in
        -l|--language)
            LANG="$2"
            shift 2
            ;;
        -s|--size)
            SIZE="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            ;;
    esac
done

# Validate size
if [[ ! " ${MODEL_SIZES[*]} " =~ " ${SIZE} " ]]; then
    echo "Error: Invalid size '$SIZE'. Valid sizes are: ${MODEL_SIZES[*]}"
    exit 1
fi

# Construct model URL based on language and size
if [[ "$LANG" == "en-us" ]]; then
    if [[ "$SIZE" == "small" ]]; then
        MODEL_URL="https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip"
    elif [[ "$SIZE" == "medium" ]]; then
        MODEL_URL="https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip"
    elif [[ "$SIZE" == "large" ]]; then
        MODEL_URL="https://alphacephei.com/vosk/models/vosk-model-en-us-0.42.zip"
    fi
else
    # For other languages, try to construct a URL based on conventions
    MODEL_URL="https://alphacephei.com/vosk/models/vosk-model-$SIZE-$LANG-0.22.zip"
    echo "Note: Using estimated URL for $LANG model: $MODEL_URL"
    echo "If download fails, check available models at https://alphacephei.com/vosk/models"
fi

# Create output directory
mkdir -p "$OUTPUT"

# Download and extract model
echo "Downloading Vosk model from: $MODEL_URL"
echo "This may take a while depending on the model size..."

if command -v unzip &> /dev/null; then
    # If unzip is available, download and extract in one step
    $DOWNLOAD_CMD "$MODEL_URL" | unzip -d "$OUTPUT" -
    
    # Rename the extracted directory to just "model"
    EXTRACTED_DIR=$(find "$OUTPUT" -maxdepth 1 -type d -name "vosk-model*" | head -n 1)
    if [[ -n "$EXTRACTED_DIR" && "$EXTRACTED_DIR" != "$OUTPUT" ]]; then
        mv "$EXTRACTED_DIR"/* "$OUTPUT/"
        rmdir "$EXTRACTED_DIR"
    fi
else
    # If unzip is not available, save the zip file first
    TEMP_ZIP="$OUTPUT/vosk_model.zip"
    $DOWNLOAD_CMD "$MODEL_URL" > "$TEMP_ZIP"
    
    echo "Downloaded to $TEMP_ZIP"
    echo "Please extract the contents to $OUTPUT manually."
    echo "After extraction, you may need to move files from the subdirectory to $OUTPUT."
    exit 0
fi

echo "Model downloaded and extracted to: $OUTPUT"
echo "You can now use this model with WhisperKey by specifying the model path: $OUTPUT" 