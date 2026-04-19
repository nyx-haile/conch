# conch

Voice interview CLI. You pick a topic; conch drives a spoken Q&A over mic + TTS
and emits a written brief. Three depths: `sketch` (Haiku), `talk` (Sonnet),
`chronicle` (Opus).

## Pipeline

- **Audio**: `cpal` mic → 16 kHz PCM
- **STT**: Deepgram streaming (cloud) or `local` (stub — see beads)
- **LLM**: Anthropic native API, or OpenRouter for DeepSeek / Llama / Gemini
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

## Config

Environment (see `.env.example`):

- `ANTHROPIC_API_KEY` — Anthropic native
- `OPENROUTER_API_KEY` — non-Anthropic providers
- `DEEPGRAM_API_KEY` — streaming STT
- `ELEVENLABS_API_KEY` — premium TTS

## Credits & inspiration

conch is independent work. Related projects studied during design:

- **[ghost-pepper](https://github.com/matthartman/ghost-pepper)** (MIT, Matt
  Hartman) — macOS-only local dictation with WhisperKit + LLM.swift. Original
  "privacy-first local voice" framing that informs conch's local backend goals.
- **[pepper-x](https://github.com/obra/pepper-x)** (MIT, Jesse Vincent) —
  Linux/GNOME Rust port of the ghost-pepper pattern. Source of design
  inspiration for conch's planned local STT (via `parakeet-rs`) and
  transcription corrections store. If verbatim code is borrowed, its MIT
  license text will be vendored under `LICENSES/pepper-x-MIT.txt` and the
  source files tagged with the upstream copyright.

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

TBD — no `LICENSE` file committed yet. Until one is added, treat the source as
"all rights reserved" for redistribution purposes. Internal use and
contributions welcome.
