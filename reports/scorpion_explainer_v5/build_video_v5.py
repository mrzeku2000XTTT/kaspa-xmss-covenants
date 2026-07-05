#!/usr/bin/env python3
import subprocess, json, os

os.chdir("/app/reports")

W, H = 1920, 1080
FPS = 30
FADE = 0.6

boundaries = json.load(open("v5_boundaries.json"))
images = [
    "v5_assets/00_intro.png",
    "v5_assets/01_identity.png",
    "v5_assets/02_xmss.png",
    "v5_assets/03_privacy.png",
    "v5_assets/04_game.png",
    "v5_assets/05_route.png",
    "v5_assets/06_reproduction.png",
    "v5_assets/07_memory.png",
    "v5_assets/08_escrow.png",
    "v5_assets/09_metabolism.png",
    "v5_assets/10_outro.png",
]
N = len(images)
assert len(boundaries) == N + 1

AUDIO_OFFSET = 0.15
TAIL = 1.05
seg_durs = [boundaries[i+1] - boundaries[i] for i in range(N)]
# pad each segment slightly so the crossfades have material to consume, and start clip
# a touch before its caption to avoid a hard cut against the previous fade
PAD = FADE
durs = [seg_durs[i] + PAD for i in range(N)]
durs[-1] += TAIL

total_out = boundaries[-1] - boundaries[0] + AUDIO_OFFSET + TAIL

inputs = []
for i, img in enumerate(images):
    d = durs[i]
    n_frames = int(d * FPS) + 2
    inputs += ["-loop", "1", "-t", f"{d:.3f}", "-i", img]

filt = []
zoom_directions = ["in", "out"] * ((N // 2) + 1)
for i in range(N):
    d = durs[i]
    n_frames = int(d * FPS) + 2
    zdir = zoom_directions[i]
    if zdir == "in":
        zexpr = "zoom+0.0009"
        zmin, zmax = "1.0", "1.18"
    else:
        zexpr = "if(eq(on,1),1.18,zoom-0.0009)"
        zmin, zmax = "1.0", "1.18"
    # Ken Burns: slow zoom + slight pan, centered
    filt.append(
        f"[{i}:v]scale=2400:1350,zoompan=z='min(max({zexpr},1.0),1.18)':"
        f"x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d={n_frames}:s={W}x{H}:fps={FPS},"
        f"setsar=1[z{i}];"
    )

# crossfade chain
filt.append(f"[z0][z1]xfade=transition=fade:duration={FADE}:offset={seg_durs[0]:.3f}[x1];")
cum = seg_durs[0]
for i in range(1, N - 1):
    cum += seg_durs[i]
    filt.append(f"[x{i}][z{i+1}]xfade=transition=fade:duration={FADE}:offset={cum:.3f}[x{i+1}];")

last_label = f"x{N-1}"
filt.append(f"[{last_label}]subtitles=v5_captions.ass[vout]")

filter_complex = "".join(filt)

cmd = [
    "ffmpeg", "-y"
] + inputs + [
    "-i", "v5_mixed_audio.m4a",
    "-filter_complex", filter_complex,
    "-map", "[vout]", "-map", f"{N}:a",
    "-t", f"{total_out:.3f}",
    "-r", str(FPS), "-c:v", "libx264", "-preset", "medium", "-crf", "19",
    "-c:a", "aac", "-b:a", "192k",
    "-pix_fmt", "yuv420p",
    "-movflags", "+faststart",
    "Scorpion_Brain_Explainer_v5.mp4",
]

print("total_out:", total_out)
print("running ffmpeg...")
subprocess.run(cmd, check=True)
print("done -> Scorpion_Brain_Explainer_v5.mp4")
