"""
Faceless Video Studio — pipeline engine.

Script (chat) -> per-scene TTS (duration drives timing) -> per-scene image ->
ffmpeg Ken-Burns + crossfade assembly + ambient music bed -> final MP4.

The visual style, timing strategy, and assembly are identical to the validated
sandbox build; only the generation calls go through the pluggable provider.
"""
from __future__ import annotations

import asyncio
import json
import subprocess
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Awaitable, Callable

from .providers import get_provider, OPENAI_VOICES, MediaProvider

ROOT = Path(__file__).resolve().parent.parent
JOBS_DIR = ROOT / "jobs"
JOBS_DIR.mkdir(exist_ok=True)

VISUAL_STYLE = (
    "Cinematic atmospheric fine-art photograph, 16:9 anamorphic widescreen. "
    "Moody, introspective, symbolic. A lone solitary human figure small in the "
    "frame, dwarfed by a vast empty environment (rainy city street, foggy shore, "
    "empty room, endless road, starlit field, dim hallway, snow, ocean). "
    "Soft volumetric god-rays, deep shadows, shallow depth of field, subtle 35mm "
    "film grain, gentle lens flare. Muted desaturated palette: deep teal, charcoal, "
    "slate blue, dusty rose, warm amber accent light. Generous negative space. "
    "No visible faces in sharp detail, no text, no logos, no watermark. "
    "Photorealistic, cinematic color grade."
)

SCRIPT_SYSTEM = """You are a master scriptwriter for calm, cinematic YouTube videos in the style of reflective psychology / emotional-insight channels (think "The Psychology of People Who...").

Voice & tone:
- Second person ("you"), warm, intimate, never preachy or clinical.
- Gentle metaphors, soft repetition, emotional truth.
- Open with a resonant hook line that names a specific feeling.
- Each scene carries ONE idea, paced for slow narration. End with a redemptive, hopeful turn.

Rules:
- Narration reads naturally when spoken aloud. 2-4 sentences per scene.
- NO hashtags, emojis, brackets, or stage directions inside the narration text.
- The "visual" field describes ONLY what the camera sees — concrete, photographic. Never put text in the image.

Return STRICT JSON only (no markdown fences): {"title": str, "scenes": [{"narration": str, "visual": str}, ...]}"""

ProgressCb = Callable[[str, dict], Awaitable[None]]


@dataclass
class Scene:
    narration: str
    visual: str
    audio_path: str = ""
    image_path: str = ""
    duration: float = 0.0


@dataclass
class JobResult:
    title: str = ""
    scenes: list[Scene] = field(default_factory=list)
    video_path: str = ""
    total_duration: float = 0.0


def _run(cmd: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True)


def _ffprobe_duration(path: str) -> float:
    out = _run(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1", path])
    if out.returncode != 0:
        raise RuntimeError(f"ffprobe failed: {out.stderr}")
    return float(out.stdout.strip())


# --------------------------------------------------------------------------- #
# Generation (provider-backed)
# --------------------------------------------------------------------------- #
async def generate_script(provider: MediaProvider, topic: str, n_scenes: int) -> dict:
    system = SCRIPT_SYSTEM + f"\nExactly {n_scenes} scenes."
    user = f"Topic / theme: {topic}\n\nWrite the script now. Return ONLY the JSON object."
    raw = await asyncio.to_thread(provider.chat, system, user)
    text = raw.strip()
    if text.startswith("```"):
        text = text.split("```json")[-1].split("```")[0] if "```json" in text else text.split("```")[1]
    if not text.startswith("{"):
        s, e = text.find("{"), text.rfind("}")
        if s != -1 and e != -1:
            text = text[s:e + 1]
    data = json.loads(text)
    data["scenes"] = data["scenes"][:n_scenes]
    return data


async def make_scene_audio(provider, text, voice, out_path) -> float:
    audio_bytes = await asyncio.to_thread(provider.tts, text, voice)
    out_path.write_bytes(audio_bytes)
    return _ffprobe_duration(str(out_path))


async def make_scene_image(provider, visual, out_path) -> None:
    prompt = f"{visual}. {VISUAL_STYLE}"
    img_bytes = await asyncio.to_thread(provider.image, prompt)
    out_path.write_bytes(img_bytes)


# --------------------------------------------------------------------------- #
# ffmpeg assembly (validated)
# --------------------------------------------------------------------------- #
def _ken_burns_segment(image: str, dur: float, out: Path) -> None:
    d = max(dur, 0.1)
    fps = 30
    frames = max(int(round(d * fps)), 2)
    zoom_total = 0.08
    vf = (
        "scale=2560:1440:force_original_aspect_ratio=increase,"
        f"zoompan=z='1+{zoom_total}*on/{frames}':"
        f"x='iw/2-(iw/zoom/2)+sin(on/40)*24':"
        f"y='ih/2-(ih/zoom/2)+cos(on/35)*16':"
        f"d={frames}:s=1920x1080:fps={fps},"
        "setsar=1,format=yuv420p,"
        f"fade=t=in:st=0:d=0.35,fade=t=out:st={max(d-0.35,0.0)}:d=0.35"
    )
    cmd = ["ffmpeg", "-y", "-loop", "1", "-i", image, "-vf", vf, "-an",
           "-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p",
           "-r", str(fps), "-t", f"{d}", str(out)]
    res = _run(cmd)
    if res.returncode != 0:
        raise RuntimeError(f"ken-burns failed: {res.stderr[-1200:]}")


def _make_music_bed(dur: float, out: Path) -> None:
    d = max(dur, 1.0)
    inputs = []
    for f in ["110.00", "164.81", "220.00"]:
        inputs += ["-f", "lavfi", "-i", f"sine=frequency={f}:duration={d}"]
    fc = ("[0:a][1:a][2:a]amerge=inputs=3,lowpass=f=700,"
          "aecho=0.8:0.7:20:0.3,volume=0.22,"
          f"afade=t=in:st=0:d=1.0,afade=t=out:st={max(d-1.5,0.0)}:d=1.5[a]")
    res = _run(["ffmpeg", "-y", *inputs, "-filter_complex", fc, "-map", "[a]",
                "-ac", "2", "-ar", "44100", str(out)])
    if res.returncode != 0:
        raise RuntimeError(f"music bed failed: {res.stderr[-1200:]}")


def assemble_video(scenes: list[Scene], job_dir: Path, music: bool = True) -> Path:
    segs, audios = [], []
    for i, sc in enumerate(scenes):
        seg = job_dir / f"seg_{i}.mp4"
        _ken_burns_segment(sc.image_path, sc.duration, seg)
        segs.append(seg)
        audios.append(Path(sc.audio_path))

    xv = job_dir / "video.mp4"
    inputs: list[str] = []
    for s in segs:
        inputs += ["-i", str(s)]
    fc, prev, xfade_d, accum = [], "[0:v]", 0.6, _ffprobe_duration(str(segs[0]))
    for i in range(1, len(segs)):
        di = _ffprobe_duration(str(segs[i]))
        offset = max(accum - xfade_d, 0.0)
        nxt = f"v{i}"
        fc.append(f"{prev}[{i}:v]xfade=transition=fade:duration={xfade_d}:offset={offset:.3f}[{nxt}]")
        prev = f"[{nxt}]"
        accum = accum + di - xfade_d
    fc.append(f"{prev}format=yuv420p[vout]")
    res = _run(["ffmpeg", "-y", *inputs, "-filter_complex", ";".join(fc),
                "-map", "[vout]", "-c:v", "libx264", "-preset", "veryfast",
                "-pix_fmt", "yuv420p", "-r", "30", str(xv)])
    if res.returncode != 0:
        raise RuntimeError(f"video xfade failed: {res.stderr[-1500:]}")

    ainputs: list[str] = []
    for a in audios:
        ainputs += ["-i", str(a)]
    afc, aprev, across_d = [], "[0:a]", 0.25
    for i in range(1, len(audios)):
        nxt = f"a{i}"
        afc.append(f"{aprev}[{i}:a]acrossfade=d={across_d}[{nxt}]")
        aprev = f"[{nxt}]"
    afc.append(f"{aprev}anull[aout]")
    narr = job_dir / "narration.wav"
    res = _run(["ffmpeg", "-y", *ainputs, "-filter_complex", ";".join(afc),
                "-map", "[aout]", "-ac", "2", "-ar", "44100", str(narr)])
    if res.returncode != 0:
        raise RuntimeError(f"narration concat failed: {res.stderr[-1500:]}")

    total = _ffprobe_duration(str(narr))
    final = job_dir / "final.mp4"
    if music:
        bed = job_dir / "music.wav"
        _make_music_bed(total + 1.0, bed)
        res = _run(["ffmpeg", "-y", "-i", str(xv), "-i", str(narr), "-i", str(bed),
                    "-filter_complex",
                    "[2:a]volume=0.10[bg];[1:a][bg]amix=inputs=2:duration=first:dropout_transition=0[aout]",
                    "-map", "0:v", "-map", "[aout]", "-c:v", "copy", "-c:a", "aac",
                    "-b:a", "192k", "-shortest", str(final)])
    else:
        res = _run(["ffmpeg", "-y", "-i", str(xv), "-i", str(narr),
                    "-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac",
                    "-b:a", "192k", "-shortest", str(final)])
    if res.returncode != 0:
        raise RuntimeError(f"final mux failed: {res.stderr[-1500:]}")
    return final


# --------------------------------------------------------------------------- #
async def render_job(topic: str, voice: str, n_scenes: int,
                     progress: ProgressCb, job_id: str, music: bool,
                     provider: MediaProvider) -> JobResult:
    job_dir = JOBS_DIR / job_id
    job_dir.mkdir(parents=True, exist_ok=True)

    await progress("script", {"status": "Writing script…"})
    data = await generate_script(provider, topic, n_scenes)
    title = data.get("title", topic)
    scenes = [Scene(narration=s["narration"], visual=s["visual"]) for s in data["scenes"]]
    await progress("script_done", {"title": title,
                                  "scenes": [{"narration": s.narration, "visual": s.visual} for s in scenes]})

    for i, sc in enumerate(scenes):
        await progress("audio", {"index": i, "total": len(scenes),
                                 "status": f"Recording voiceover (scene {i+1}/{len(scenes)})…"})
        ap = job_dir / f"scene_{i}.mp3"
        sc.duration = await make_scene_audio(provider, sc.narration, voice, ap)
        sc.audio_path = str(ap)

        await progress("image", {"index": i, "total": len(scenes),
                                 "status": f"Creating visual (scene {i+1}/{len(scenes)})…"})
        ip = job_dir / f"scene_{i}.png"
        await make_scene_image(provider, sc.visual, ip)
        sc.image_path = str(ip)
        await progress("image_done", {"index": i, "image_url": f"/api/jobs/{job_id}/scene_{i}.png"})

    await progress("assemble", {"status": "Assembling final video with motion & music…"})
    final = await asyncio.to_thread(assemble_video, scenes, job_dir, music)
    total_dur = _ffprobe_duration(str(final))
    await progress("done", {"video_url": f"/api/jobs/{job_id}/video.mp4", "duration": round(total_dur, 2)})
    return JobResult(title=title, scenes=scenes, video_path=str(final), total_duration=total_dur)
