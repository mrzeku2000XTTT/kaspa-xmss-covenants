# Faceless Video Studio

Turn one line into a cinematic, narrated video. The studio writes a reflective
script, records a calm voiceover, paints a visual for each scene, and assembles a
timed, scored film — with the visuals locked to the narration by construction.

Inspired by calm, reflective psychology channels (e.g. "The Psychology of People
Who..."). Self-host it, bring your own API key, and mass-produce videos from a
topics file.

![pipeline](https://img.shields.io/badge/pipeline-script→voice→image→film-teal) ![stack](https://img.shields.io/badge/stack-FastAPI%20%2B%20OpenAI%20%2B%20ffmpeg-blue) ![license](https://img.shields.io/badge/license-MIT-green)

---

## How it works

1. **Script** — an LLM writes a reflective, second-person script and splits it into scenes, each with a narration line and a concrete visual description.
2. **Voiceover** — each scene's narration is sent to TTS. The returned audio's **real duration becomes that scene's length**, so visuals and narration stay in sync automatically — no guesswork, no forced alignment.
3. **Visuals** — one image per scene, all sharing a consistent cinematic visual bible (lone figure, dark teal/charcoal, volumetric light, film grain, no text).
4. **Assembly** — ffmpeg applies a smooth Ken Burns zoom/pan to every image for exactly its scene duration, crossfades between scenes, crossfades the narration, and mixes in a generated ambient music bed.

```
topic  ─►  script(JSON)  ─►  per-scene TTS (duration)  ─►  per-scene image
                                                            │
                          ◄── ffmpeg: Ken Burns + xfade + music bed ──┘
                          │
                          └─►  final.mp4 (1920×1080, 30fps)
```

## Quick start

### 1. Install

```bash
git clone <your-repo-url> faceless-video-studio
cd faceless-video-studio
python -m venv .venv && source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -r requirements.txt
# ffmpeg is required — macOS: brew install ffmpeg · Ubuntu: sudo apt install ffmpeg
```

### 2. Add your OpenAI key

```bash
cp .env.example .env
echo "OPENAI_API_KEY=sk-..." >> .env
```

One OpenAI key powers script writing, voiceover, and image generation. No key?
The app runs in **mock mode** so you can try the full flow for free (placeholder
audio/images).

### 3. Run the dashboard

```bash
uvicorn app.server:app --host 0.0.0.0 --port 8000
```

Open http://localhost:8000, type a topic, pick a voice, and hit **Generate**.
The KCC20 Wallet hammer app can also drive this server: Profile → hammer →
Faceless Studio → Engine **Python studio server** (http://127.0.0.1:8000).
Watch the script, voiceover, and visuals come together live — then download the
MP4. You can also paste an API key directly in the dashboard (BYOK); it's sent
per-request in a header and never stored.

## Mass production (batch)

Render many videos from a topics file (one topic per line, `#` = comment):

```bash
python -m app.batch --topics topics.txt --voice nova --scenes 6 --concurrency 1
```

- Writes `output/results.csv` + a per-topic `manifest.json`.
- **Resumable**: skips topics already rendered unless `--force`.
- Free dry run: `MOCK=1 python -m app.batch --topics topics.txt`
- ⚠️ Keep `--concurrency 1` unless you understand the cost. Each video calls image
  generation once per scene.

## Docker

```bash
docker build -t faceless-video-studio .
docker run -p 8000:8000 -e OPENAI_API_KEY=sk-... -v "$PWD/jobs:/app/jobs" faceless-video-studio
```

Deployable as-is to Railway, Render, Fly, or any container host. For long
sessions or many videos, mount a volume (or wire up S3/R2) so renders persist.

## Configuration

| Env var          | Purpose                                              | Default        |
| ---------------- | ---------------------------------------------------- | -------------- |
| `OPENAI_API_KEY` | Script + TTS + images. Unset = mock mode.            | _(unset)_      |
| `OPENAI_BASE_URL`| OpenAI-compatible endpoint (Azure, proxy, etc.)     | _(OpenAI)_     |
| `MOCK`           | Force mock mode even with a key.                     | _(unset)_      |

Defaults live in `app/providers.py`:

| Stage | Model        | Notes                                  |
| ----- | ------------ | -------------------------------------- |
| Script| `gpt-4o-mini`| Cheap, strong reflective writing.     |
| TTS   | `tts-1`      | Voices: nova, echo, onyx, alloy, shimmer, fable. |
| Image | `gpt-image-1`| Size 1536×1024, quality `low` (cheapest). Raise quality for sharper visuals. |

Want a different voice or script model? Edit those constants — the provider
layer is isolated so you can swap in Anthropic, ElevenLabs, or Google later
without touching the pipeline.

## Project structure

```
faceless-video-studio/
├── app/
│   ├── providers.py     # OpenAI + mock provider (swap providers here)
│   ├── pipeline.py      # script → TTS → image → ffmpeg assembly
│   ├── server.py        # FastAPI + SSE + BYOK
│   ├── batch.py         # mass-production CLI (topics file → many videos)
│   └── public/          # dashboard (index.html, style.css, app.js)
├── topics.txt          # sample batch input
├── requirements.txt
├── Dockerfile          # ffmpeg baked in
├── .env.example
└── README.md
```

## Cost & safety notes

- A 7-scene video at default quality costs roughly a few cents in image
  generation plus a fraction of a cent for TTS and script. Start small.
- Your API key is never stored server-side when using BYOK; request headers are
  not logged. Prefer the `OPENAI_API_KEY` env var for shared/hosted deployments.
- Generated images and ambient music are synthetic. If you publish to YouTube,
  review YouTube's policies and keep the auto-generated (royalty-safe) music bed
  or swap in your own licensed track. You are responsible for your output.
