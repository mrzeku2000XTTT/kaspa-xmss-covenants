"""
Provider layer for Faceless Video Studio.

Default: OpenAI (one API key powers script writing, voiceover, and image
generation). A mock provider is used automatically when no key is present so
you can try the full app flow (SSE, queue, ffmpeg, download) without spending
anything.

Provider adapters are isolated here so Anthropic / ElevenLabs / Google can be
added later without touching the pipeline.
"""
from __future__ import annotations

import base64
import hashlib
import os
import subprocess
import urllib.request
from pathlib import Path
from typing import Protocol


# OpenAI TTS voices mapped to friendly labels.
OPENAI_VOICES = {
    "nova":   "nova — warm, natural (female)",
    "echo":   "echo — calm, steady (male)",
    "onyx":   "onyx — deep, professional (male)",
    "alloy":  "alloy — neutral, balanced",
    "shimmer": "shimmer — soft, warm (female)",
    "fable":  "fable — expressive, British (neutral)",
}

DEFAULT_CHAT_MODEL = "gpt-4o-mini"
DEFAULT_TTS_MODEL = "tts-1"
DEFAULT_IMAGE_MODEL = "gpt-image-1"
DEFAULT_IMAGE_SIZE = "1536x1024"   # 3:2, scaled/cropped to 16:9 in assembly
DEFAULT_IMAGE_QUALITY = "low"      # low | medium | high | auto


class MediaProvider(Protocol):
    def chat(self, system: str, user: str, **kw) -> str: ...
    def tts(self, text: str, voice: str, **kw) -> bytes: ...
    def image(self, prompt: str, **kw) -> bytes: ...


# --------------------------------------------------------------------------- #
# OpenAI provider
# --------------------------------------------------------------------------- #
class OpenAIProvider:
    def __init__(self, api_key: str | None = None, base_url: str | None = None):
        from openai import OpenAI
        self.client = OpenAI(
            api_key=api_key or os.getenv("OPENAI_API_KEY"),
            base_url=base_url or os.getenv("OPENAI_BASE_URL") or None,
        )

    def chat(self, system, user, model=DEFAULT_CHAT_MODEL, max_tokens=4096, temperature=0.9) -> str:
        r = self.client.chat.completions.create(
            model=model, max_tokens=max_tokens, temperature=temperature,
            response_format={"type": "json_object"},
            messages=[{"role": "system", "content": system}, {"role": "user", "content": user}],
        )
        return r.choices[0].message.content or ""

    def tts(self, text, voice="nova", model=DEFAULT_TTS_MODEL) -> bytes:
        raw = self.client.audio.speech.with_raw_response.create(
            model=model, voice=voice, input=text, response_format="mp3",
        )
        return raw.content

    def image(self, prompt, model=DEFAULT_IMAGE_MODEL, size=DEFAULT_IMAGE_SIZE,
              quality=DEFAULT_IMAGE_QUALITY) -> bytes:
        r = self.client.images.generate(model=model, prompt=prompt, n=1,
                                        size=size, quality=quality)
        d = r.data[0]
        if getattr(d, "b64_json", None):
            return base64.b64decode(d.b64_json)
        if getattr(d, "url", None):
            return urllib.request.urlopen(d.url).read()
        raise RuntimeError("image generation returned no data")


# --------------------------------------------------------------------------- #
# Mock provider — no API key required. Placeholder audio + images so the full
# app flow can be tested for free. Outputs are NOT real AI generation.
# --------------------------------------------------------------------------- #
def _word_count(t: str) -> int:
    return len(t.split())


class MockProvider:
    """Generates silent audio (duration estimated from text) and colored PNGs."""
    def __init__(self):
        self._warned = False

    def _note(self):
        if not self._warned:
            print("[mock] No OPENAI_API_KEY set — using placeholder generation. "
                  "Outputs are not real AI content.")
            self._warned = True

    def chat(self, system, user, **kw) -> str:
        self._note()
        import json
        # produce a tiny canned script so the UI/pipeline can run end-to-end
        scenes = []
        words = ["stillness", "drift", "threshold", "horizon", "echo", " ember", "return"]
        for i in range(7):
            scenes.append({
                "narration": f"This is scene {i+1}. A mock placeholder line about {words[i % len(words)]}. "
                             f"Replace with a real OpenAI key to generate the actual script.",
                "visual": f"A lone figure in a vast empty space, scene {i+1}, cinematic, moody, dark teal",
            })
        return json.dumps({"title": "Mock Preview — add an OpenAI key for real output", "scenes": scenes})

    def tts(self, text, voice="nova", **kw) -> bytes:
        self._note()
        # ~2.4 words per second, min 3s, capped to keep things snappy
        dur = max(3.0, min(14.0, _word_count(text) / 2.4))
        return _silent_mp3(dur)

    def image(self, prompt, **kw) -> bytes:
        self._note()
        # deterministic hue from the prompt so scenes differ
        h = int(hashlib.md5(prompt.encode()).hexdigest(), 16) % 360
        return _color_png(h)


def _silent_mp3(duration: float) -> bytes:
    import tempfile
    with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as f:
        path = f.name
    subprocess.run(
        ["ffmpeg", "-y", "-f", "lavfi", "-i", f"anullsrc=r=44100:cl=stereo",
         "-t", f"{duration}", path],
        check=True, capture_output=True)
    data = Path(path).read_bytes()
    Path(path).unlink(missing_ok=True)
    return data


def _color_png(hue: int) -> bytes:
    import tempfile
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
        path = f.name
    color = f"0x{int(hue/360*255):02x}{int((1-hue/360)*255):02x}40"
    subprocess.run(
        ["ffmpeg", "-y", "-f", "lavfi", "-i", f"color=c={color}:s=1536x1024:d=1",
         "-frames:v", "1", path], check=True, capture_output=True)
    data = Path(path).read_bytes()
    Path(path).unlink(missing_ok=True)
    return data


# --------------------------------------------------------------------------- #
def get_provider(api_key: str | None = None) -> MediaProvider:
    """Resolve a provider. Honors OPENAI_PROVIDER and MOCK env vars."""
    if os.getenv("MOCK", "").lower() in ("1", "true", "yes"):
        return MockProvider()
    key = api_key or os.getenv("OPENAI_API_KEY")
    if key:
        return OpenAIProvider(api_key=key)
    return MockProvider()
