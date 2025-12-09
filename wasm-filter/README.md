# WASM Filter

A WebAssembly-based content filtering proxy for Bord, a social media platform. Provides multi-layer content filtering: keyword-based detection + optional ML-powered sentiment analysis.

## Features

- **Fast keyword-based filtering** - Forbidden words detection
- **LLM-powered sentiment analysis** (optional) - Ollama-based analysis for more nuanced detection
- **ML-powered sentiment analysis** (optional) - Tract ONNX inference for sentiment classification
- **Zero-latency inference** - Model runs in-process, no external API calls
- **Configurable** - Enable/disable ML and LLM classification via `config.toml`

## Setup

### 1. Start main Bord app on different port

```bash
cd ../
spin up --listen 127.0.0.1:3001
```

### 2. Download ML Model (Optional)

If using ML-based sentiment filtering, download the Xenova DistilBERT sentiment model:

```bash
# Create models directory
mkdir -p models

# Download ONNX model
wget -O models/model.onnx \
  https://huggingface.co/Xenova/distilbert-base-uncased-finetuned-sst-2-english/resolve/main/onnx/model.onnx

# Download vocabulary
wget -O models/vocab.txt \
  https://huggingface.co/Xenova/distilbert-base-uncased-finetuned-sst-2-english/resolve/main/vocab.txt

# Download tokenizer config (optional, for reference)
wget -O models/tokenizer_config.json \
  https://huggingface.co/Xenova/distilbert-base-uncased-finetuned-sst-2-english/resolve/main/tokenizer_config.json
```

**Model Details:**

- **Name:** Xenova DistilBERT SST-2 (ONNX-optimized)
- **Task:** Binary sentiment classification (positive/negative)
- **Tract compatible:** Yes (no unsupported Attention ops)

### 3. Configure Filtering

Edit `config.toml`:

```toml
# Enable/disable ML classification
enable_llm = false      # Ollama LLM (slower, API-based)
enable_tract = true     # Tract ONNX (faster, in-process)

[llm]
address = "http://127.0.0.1:11434"
model = "mistral"
temperature = 0.7

[llm_prompt]
sentiment_analysis = "..."

[policy]
sentiment_score_threshold = 0.2  # Block if score < 0.2 (very negative)
```

## Running

```bash
spin up
```

The filter proxy runs on `http://localhost:3000` and forwards requests to the main Bord app on `http://localhost:3001`.

## How It Works

### Filtering Pipeline

```bash
POST /posts {content: "..."}
    ↓
1. Forbidden words check (keyword blacklist)
    ↓
2. ML sentiment analysis (Tract ONNX) [if enabled]
    ├─ Tokenize input text
    ├─ Run inference
    ├─ Extract sentiment score (0.0-1.0)
    └─ Decision: negative sentiment → block or flag
    ↓
3. Forward to main app (if passes filters)
```

### Sentiment Score Interpretation

- **0.0-0.2:** Very negative (potentially toxic/hateful) → **Block**
- **0.2-0.4:** Negative (may flag for review)
- **0.4-0.6:** Neutral
- **0.6-1.0:** Positive → **Allow**

## Architecture

### Modules

- **src/lib.rs** - Main HTTP handler, filtering logic
- **src/tokenizer.rs** - BERT tokenizer (converts text → token IDs)
- **src/tract_model.rs** - Tract ONNX model loading & inference
- **config.toml** - Configuration (forbidden words, thresholds, ML settings)

### Dependencies

- `spin-sdk` - WebAssembly HTTP runtime
- `tract-onnx` - ONNX model inference
- `serde` - Configuration parsing

## Limitations

- **English only:** Model trained on English (SST-2). Non-English text may have poor accuracy.
- **Context-unaware:** Single-pass sentiment; doesn't understand context or nuance.
- **WASM constraints:** Model size + dependencies affect binary size.

## References

- [DistilBERT Paper](https://arxiv.org/abs/1910.01108)
- [SST-2 Sentiment Dataset](https://nlp.stanford.edu/sentiment/)
- [Tract ONNX Runtime](https://github.com/sonos/tract)
