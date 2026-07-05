#!/usr/bin/env python3
import asyncio, json, sys
import edge_tts

TEXT_FILE = "reports/v5_narration.txt"
VOICE = "en-US-AriaNeural"   # expressive female neural voice
RATE = "+4%"
PITCH = "+0Hz"
OUT_MP3 = "reports/narration_v5.mp3"
OUT_JSON = "reports/narration_v5_words.json"

async def main():
    text = open(TEXT_FILE).read().strip()
    communicate = edge_tts.Communicate(text, VOICE, rate=RATE, pitch=PITCH, boundary="WordBoundary")
    words = []
    with open(OUT_MP3, "wb") as f:
        async for chunk in communicate.stream():
            if chunk["type"] == "audio":
                f.write(chunk["data"])
            elif chunk["type"] == "WordBoundary":
                words.append({
                    "text": chunk["text"],
                    "offset_s": chunk["offset"] / 10_000_000,
                    "duration_s": chunk["duration"] / 10_000_000,
                })
    json.dump(words, open(OUT_JSON, "w"), indent=2)
    print(f"wrote {OUT_MP3} and {OUT_JSON}, {len(words)} words")

asyncio.run(main())
