# conch

Voice interview CLI. You pick a topic; conch drives a spoken Q&A over mic + TTS
and emits a written brief. Three depths: `sketch` (Haiku), `talk` (Sonnet),
`chronicle` (Opus).

## Pipeline

- **Audio**: `cpal` mic → 16 kHz PCM
- **STT**: Deepgram streaming (cloud) or `local` (NVIDIA Nemotron 0.6B int8 via `parakeet-rs` + ONNX Runtime)
- **LLM**: OpenRouter launch gateway (Grok default + paid model catalog), Anthropic native API, OpenAI native API, or routed DeepSeek / Llama / Gemini
- **TTS**: ElevenLabs (cloud), `piper` (local), or silent text mode
- **UI**: `ratatui` TUI with live partials + waveform
- **Sessions**: transcripts + briefs persisted per session; re-exportable

## Usage

```bash
conch sketch "project topic"      # fast rough brief
conch talk "project topic"        # default
conch chronicle "project topic"   # deep research
conch sessions                    # list past sessions
conch export <session-id>         # re-emit outputs
conch test                        # scripted smoke test, no mic
```

Backend overrides: `--stt deepgram|local`, `--tts elevenlabs|local|text`,
`--no-tts` (silent).

### Local STT model

`--stt local` loads NVIDIA's Nemotron streaming 0.6B int8 via
[`parakeet-rs`](https://crates.io/crates/parakeet-rs) on CPU (ONNX Runtime).
On first use conch downloads three files (~250 MB total) from
[`smcleod/nemotron-speech-streaming-en-0.6b-int8`](https://huggingface.co/smcleod/nemotron-speech-streaming-en-0.6b-int8)
into `~/.conch/models/parakeet-nemotron-streaming-en-0.6b/`:

- `encoder.onnx`
- `decoder_joint.onnx`
- `tokenizer.model`

Override the cache path with `CONCH_PARAKEET_MODEL_DIR=/path/to/bundle`.
Disable auto-download with `CONCH_PARAKEET_NO_DOWNLOAD=1` (in that case
populate the directory yourself, e.g. via
`hf download smcleod/nemotron-speech-streaming-en-0.6b-int8 --local-dir <dir>`).
Once the bundle is cached conch runs fully offline.

## Config

Environment (see `.env.example`):

- `ANTHROPIC_API_KEY` — Anthropic native
- `OPENAI_API_KEY` — OpenAI native
- `OPENAI_API_KEY_FILE` — optional file path containing only the OpenAI key (supports `~/...`)
- `OPENROUTER_API_KEY` — OpenRouter launch gateway (`CONCH_PROVIDER=openrouter` or `grok`) and routed DeepSeek / Llama / Gemini
- `CONCH_OUTPUT_DEVICE` — optional substring match for the live playback device/driver (e.g. `pulse`, `pipewire`, `hdmi`)
- `CONCH_INPUT_DEVICE` — optional substring match for the live mic input device (e.g. `mic`, `headset`, `digital`)
- `DEEPGRAM_API_KEY` — streaming STT
- `CONCH_STT` — STT backend: `local` (default) or `deepgram`
- `ELEVENLABS_API_KEY` — premium TTS
- `CONCH_PIPER_BIN` — local Piper executable override (default `piper-tts`)
- `CONCH_PIPER_MODEL` — local Piper `.onnx` voice path; sibling `.onnx.json` must exist


### OpenRouter model gateway

Use `CONCH_PROVIDER=openrouter` (or `grok`) to route through the launch
OpenRouter catalog. The default low-cost/free-tier user model is Grok 4 Fast
(`x-ai/grok-4-fast`); deeper modes use paid/BYOK catalog entries and should be
gated by server-side entitlements in the web launch path.

## Credits & inspiration

conch is independent work. Related projects studied during design:

- **[ghost-pepper](https://github.com/matthartman/ghost-pepper)** (MIT, Matt
  Hartman) — macOS-only local dictation with WhisperKit + LLM.swift. Original
  "privacy-first local voice" framing that informs conch's local backend goals.
- **[pepper-x](https://github.com/obra/pepper-x)** (MIT, Jesse Vincent) —
  Linux/GNOME Rust port of the ghost-pepper pattern. The 560ms streaming
  chunk size, `transcribe_chunk` → `reset()` loop shape, and the pinned
  Nemotron int8 bundle URLs in conch's local STT backend follow the
  reference design in its `pepperx-asr` crate. Upstream MIT text is
  vendored at `LICENSES/pepper-x-MIT.txt` and referenced from
  `src/stt/local.rs`.

Models & services (not bundled; fetched on demand when the relevant backend is
selected):

- **NVIDIA Parakeet / Nemotron** via [`parakeet-rs`](https://crates.io/crates/parakeet-rs)
- **Qwen** (Alibaba) — candidate local cleanup LLM
- **Piper** (Rhasspy) — local TTS
- **Whisper** (OpenAI) — referenced, not currently wired
- **Anthropic Claude**, **Deepgram**, **ElevenLabs**, **OpenRouter** — cloud
  services via their official APIs

Rust dependency tree in `Cargo.lock` lists all transitive attributions.

## License

MIT — see [`LICENSE`](LICENSE). Vendored upstream attributions live in
[`LICENSES/`](LICENSES).
