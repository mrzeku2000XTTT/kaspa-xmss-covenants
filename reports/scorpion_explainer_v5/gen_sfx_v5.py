#!/usr/bin/env python3
"""Synthesize SFX for the Scorpion explainer video: ambient hum, transition blip, heartbeat thump."""
import numpy as np
import wave

SR = 44100

def write_wav(path, samples, sr=SR):
    samples = np.clip(samples, -1.0, 1.0)
    pcm = (samples * 32767).astype(np.int16)
    with wave.open(path, "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())

# ---- Ambient hum: low sci-fi drone, ~150s, two detuned low sines + slow tremolo ----
dur = 150.0
t = np.linspace(0, dur, int(SR * dur), endpoint=False)
hum = 0.5 * np.sin(2 * np.pi * 55 * t) + 0.35 * np.sin(2 * np.pi * 82.5 * t)
tremolo = 0.7 + 0.3 * np.sin(2 * np.pi * 0.07 * t)
hum = hum * tremolo
# gentle fade in/out
fade_len = int(SR * 2)
env = np.ones_like(hum)
env[:fade_len] = np.linspace(0, 1, fade_len)
env[-fade_len:] = np.linspace(1, 0, fade_len)
hum = hum * env * 0.05  # low volume ambient bed
write_wav("reports/v5_assets/sfx_ambient_hum.wav", hum)

# ---- Transition blip: quick upward chirp 400Hz->1400Hz, 0.22s, with decay envelope ----
bdur = 0.22
bt = np.linspace(0, bdur, int(SR * bdur), endpoint=False)
freq = np.linspace(400, 1400, bt.size)
phase = 2 * np.pi * np.cumsum(freq) / SR
blip = np.sin(phase)
benv = np.exp(-bt * 14)  # fast decay
blip = blip * benv * 0.35
write_wav("reports/v5_assets/sfx_blip.wav", blip)

# ---- Heartbeat thump: two quick low sine bursts (lub-dub), ~0.5s total ----
def thump_pulse(freq, dur_s, amp):
    tt = np.linspace(0, dur_s, int(SR * dur_s), endpoint=False)
    sig = np.sin(2 * np.pi * freq * tt)
    env = np.exp(-tt * 28)
    return sig * env * amp

lub = thump_pulse(60, 0.18, 0.9)
gap = np.zeros(int(SR * 0.12))
dub = thump_pulse(50, 0.16, 0.7)
heartbeat = np.concatenate([lub, gap, dub])
write_wav("reports/v5_assets/sfx_heartbeat.wav", heartbeat)

print("SFX assets written: sfx_ambient_hum.wav, sfx_blip.wav, sfx_heartbeat.wav")
