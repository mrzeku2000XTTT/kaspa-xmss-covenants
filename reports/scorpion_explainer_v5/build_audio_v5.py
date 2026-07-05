#!/usr/bin/env python3
import subprocess, json, os

os.chdir("/app/reports")

words = json.load(open("narration_v5_words.json"))
total_dur = words[-1]["offset_s"] + words[-1]["duration_s"]

# Segment boundary times (found via word-timestamp matching against narration text)
boundaries = [0.0, 13.25, 25.36, 39.74, 53.98, 66.14, 79.28, 93.69, 105.94, 120.25, 135.4, total_dur + 1.0]
json.dump(boundaries, open("v5_boundaries.json", "w"), indent=2)

AUDIO_OFFSET = 0.15  # narration starts slightly after video start, matches earlier versions

# Heartbeat thumps during stage 9 (metabolism), boundaries[9]=120.25 -> boundaries[10]=135.4
hb_start, hb_end = boundaries[9], boundaries[10]
hb_times = []
tcur = hb_start + 0.3
while tcur < hb_end - 0.3:
    hb_times.append(tcur)
    tcur += 0.95

# Transition blips at every segment boundary (except t=0)
blip_times = [b for b in boundaries[1:-1]]

inputs = ["-i", "narration_v5.mp3", "-i", "v5_assets/sfx_ambient_hum.wav"]
filt = []
# narration delayed by AUDIO_OFFSET
filt.append(f"[0:a]adelay={int(AUDIO_OFFSET*1000)}|{int(AUDIO_OFFSET*1000)}[narr];")
filt.append("[1:a]volume=1.0[hum];")

mix_labels = ["[narr]", "[hum]"]
idx = 2
for bt in blip_times:
    inputs += ["-i", "v5_assets/sfx_blip.wav"]
    d_ms = int((bt + AUDIO_OFFSET) * 1000)
    filt.append(f"[{idx}:a]adelay={d_ms}|{d_ms},volume=0.9[blip{idx}];")
    mix_labels.append(f"[blip{idx}]")
    idx += 1

for ht in hb_times:
    inputs += ["-i", "v5_assets/sfx_heartbeat.wav"]
    d_ms = int((ht + AUDIO_OFFSET) * 1000)
    filt.append(f"[{idx}:a]adelay={d_ms}|{d_ms},volume=0.8[hb{idx}];")
    mix_labels.append(f"[hb{idx}]")
    idx += 1

n_inputs = len(mix_labels)
filt.append("".join(mix_labels) + f"amix=inputs={n_inputs}:duration=longest:normalize=0[aout]")

filter_complex = "".join(filt)

TOTAL_OUT = total_dur + AUDIO_OFFSET + 1.2

cmd = ["ffmpeg", "-y"] + inputs + [
    "-filter_complex", filter_complex,
    "-map", "[aout]",
    "-t", f"{TOTAL_OUT:.3f}",
    "-c:a", "aac", "-b:a", "192k",
    "v5_mixed_audio.m4a"
]
print("Running ffmpeg with", n_inputs, "audio inputs...")
subprocess.run(cmd, check=True)
print("done -> v5_mixed_audio.m4a, total_dur (video length target):", TOTAL_OUT)
