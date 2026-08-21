"""
Batch / mass-production CLI.

Reads a topics file (one topic per line, "#" = comment) and renders a video for
each. Writes a results.csv + manifest.json. Resumable: skips topics already done
unless --force.

Usage:
  python -m app.batch --topics topics.txt --voice nova --scenes 6 --concurrency 1
  python -m app.batch --topics topics.txt --force
  MOCK=1 python -m app.batch --topics topics.txt      # free dry run, no API key

Cost warning: each video calls image generation once per scene. Start with
--concurrency 1 and a small file until you know your spend.
"""
from __future__ import annotations

import argparse
import asyncio
import csv
import json
import os
import sys
from pathlib import Path

from .pipeline import render_job, JOBS_DIR
from .providers import get_provider


async def _render_one(topic: str, voice: str, scenes: int, music: bool, out_dir: Path,
                      results: dict) -> None:
    import shutil
    slug = topic.strip().lower().replace(" ", "_")[:40] + f"_{scenes}"
    job_dir = JOBS_DIR / slug            # where the pipeline writes assets
    out_job_dir = out_dir / slug         # where we copy the final video + manifest
    out_job_dir.mkdir(parents=True, exist_ok=True)
    manifest = out_job_dir / "manifest.json"
    final_dst = out_job_dir / "final.mp4"
    if final_dst.exists() and manifest.exists() and not results.get("force"):
        m = json.loads(manifest.read_text())
        results["rows"].append({**m, "status": "skipped"})
        print(f"[skip] {topic}")
        return

    async def cb(event, data):
        if event == "image_done" or event == "audio":
            print(f"  [{topic[:40]}] {event} {data.get('index','')}", flush=True)

    provider = get_provider()
    try:
        r = await render_job(topic=topic, voice=voice, n_scenes=scenes,
                             progress=cb, job_id=slug, music=music, provider=provider)
        final_src = Path(r.video_path)
        shutil.copy2(final_src, final_dst)
        row = {"topic": topic, "title": r.title, "scenes": scenes,
               "duration": round(r.total_duration, 2),
               "video": str(final_dst.relative_to(out_dir.parent)),
               "status": "ok"}
        manifest.write_text(json.dumps(row, indent=2))
        results["rows"].append(row)
        print(f"[ok]   {topic} -> {final_dst}")
    except Exception as e:  # noqa: BLE001
        results["rows"].append({"topic": topic, "status": "error", "error": str(e)})
        print(f"[err]  {topic}: {e}")


async def main_async(args):
    topics = [l.strip() for l in Path(args.topics).read_text().splitlines()
              if l.strip() and not l.strip().startswith("#")]
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    results = {"rows": [], "force": args.force}

    sem = asyncio.Semaphore(max(1, args.concurrency))

    async def guarded(t):
        async with sem:
            await _render_one(t, args.voice, args.scenes, args.music, out_dir, results)

    await asyncio.gather(*[guarded(t) for t in topics])

    csv_path = out_dir / "results.csv"
    with csv_path.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["topic", "title", "scenes", "duration", "video", "status", "error"])
        w.writeheader()
        for r in results["rows"]:
            w.writerow({k: r.get(k, "") for k in w.fieldnames})
    (out_dir / "manifest.json").write_text(json.dumps(results["rows"], indent=2))
    ok = sum(1 for r in results["rows"] if r["status"] == "ok")
    print(f"\nDone: {ok}/{len(topics)} ok -> {out_dir}/results.csv")


def main():
    p = argparse.ArgumentParser(description="Mass-produce faceless videos from a topics file.")
    p.add_argument("--topics", required=True, help="Path to a .txt file, one topic per line.")
    p.add_argument("--voice", default="nova")
    p.add_argument("--scenes", type=int, default=6)
    p.add_argument("--concurrency", type=int, default=1, help="Parallel renders. Keep 1 unless you understand the cost.")
    p.add_argument("--out-dir", default="output")
    p.add_argument("--no-music", dest="music", action="store_false")
    p.add_argument("--force", action="store_true", help="Re-render even if already done.")
    args = p.parse_args()
    if args.concurrency > 1:
        print("WARNING: concurrency>1 multiplies API cost and CPU. Proceed with caution.", file=sys.stderr)
    asyncio.run(main_async(args))


if __name__ == "__main__":
    main()
