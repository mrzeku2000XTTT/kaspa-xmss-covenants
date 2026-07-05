# Scorpion Brain Explainer v5

Explainer video covering the 9 build stages (10 chromosomes) of the Scorpion RISC0 brain,
modeled on an AP-style news explainer format (clean female narration, synced captions,
Ken Burns-animated background art, subtle SFX).

**Pipeline:**
1. `v5_narration.txt` — script (intro + 9 stages + outro)
2. `gen_narration_v5.py` — edge-tts (en-US-AriaNeural, female), produces narration_v5.mp3 + word-level timestamp JSON
3. `gen_sfx_v5.py` — synthesizes ambient hum, transition blip, heartbeat thump (numpy, no external assets)
4. `build_audio_v5.py` — mixes narration + hum + blips (at each stage boundary) + heartbeat (during metabolism stage) into one audio track
5. `build_captions_v5.py` — chunks word-timestamps into ASS subtitle captions, exact narration sync
6. `build_video_v5.py` — Ken Burns zoompan per background image + xfade crossfades + burned captions + mixed audio -> final mp4

Video: https://base44.app/api/apps/6a444b036408e68ec8d6f2a6/files/mp/public/6a444b036408e68ec8d6f2a6/2596ccd64_Scorpion_Brain_Explainer_v5.mp4

Background art generated per-stage (identity, XMSS, privacy, game theory, routing,
reproduction, memory, escrow, metabolism) in a consistent dark bioluminescent
teal/amber scorpion-circuit aesthetic.
