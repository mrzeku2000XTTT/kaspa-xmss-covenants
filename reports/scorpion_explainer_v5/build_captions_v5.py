#!/usr/bin/env python3
"""Build phrase-chunked ASS captions synced exactly to edge-tts word timestamps."""
import json, os

os.chdir("/app/reports")

words = json.load(open("narration_v5_words.json"))
AUDIO_OFFSET = 0.15

def fmt_time(t):
    t = max(0, t)
    h = int(t // 3600)
    m = int((t % 3600) // 60)
    s = t % 60
    return f"{h:d}:{m:02d}:{s:05.2f}"

# Group words into caption chunks: break on sentence-ending punctuation, or every ~7 words.
chunks = []
cur = []
for w in words:
    cur.append(w)
    txt = w["text"]
    ends_sentence = txt.endswith(".") or txt.endswith(":") if False else False
    # edge-tts WordBoundary text doesn't include punctuation, so break by word count + natural groups
    if len(cur) >= 6:
        chunks.append(cur)
        cur = []
if cur:
    chunks.append(cur)

# Merge tiny trailing chunk into previous if too short
if len(chunks) >= 2 and len(chunks[-1]) < 3:
    chunks[-2].extend(chunks[-1])
    chunks.pop()

header = """[Script Info]
Title: Scorpion Explainer v5 Captions
ScriptType: v4.00+
PlayResX: 1920
PlayResY: 1080
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Caption,Arial Black,64,&H00FFFFFF,&H00FFFFFF,&H00101010,&H90000000,1,0,0,0,100,100,0,0,3,3,1,2,120,120,80,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"""

lines = [header]
for chunk in chunks:
    start = chunk[0]["offset_s"] + AUDIO_OFFSET
    end = chunk[-1]["offset_s"] + chunk[-1]["duration_s"] + AUDIO_OFFSET
    text = " ".join(w["text"] for w in chunk)
    text = text.replace("{", "(").replace("}", ")")
    lines.append(f"Dialogue: 0,{fmt_time(start)},{fmt_time(end)},Caption,,0,0,0,,{text}\n")

open("v5_captions.ass", "w").write("".join(lines))
print(f"wrote v5_captions.ass with {len(chunks)} caption chunks")
