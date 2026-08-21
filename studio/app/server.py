"""
Faceless Video Studio — FastAPI server.

Auth modes:
  - OPENAI_API_KEY env var  -> recommended (hosted) mode
  - x-api-key request header -> BYOK (key never stored; not logged)

If neither is set, the mock provider runs so the app still works end-to-end.

Endpoints:
  POST /api/render            start a render -> {job_id}
  GET  /api/jobs/{id}         status / result
  GET  /api/jobs/{id}/events  SSE progress stream
  GET  /api/jobs/{id}/{file}  serve scene image / final video (range support)
  GET  /                      dashboard (public/)
"""
from __future__ import annotations

import asyncio
import json
import time
import uuid
from pathlib import Path
from typing import Optional

from fastapi import FastAPI, Header, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, StreamingResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from .providers import get_provider, OPENAI_VOICES
from .pipeline import render_job, JOBS_DIR

ROOT = Path(__file__).resolve().parent
PUBLIC = ROOT / "public"

# guardrails
MAX_ACTIVE_JOBS = 3
MAX_SCENES = 10
MAX_TOPIC_LEN = 300

app = FastAPI(title="Faceless Video Studio")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


class Job:
    def __init__(self, params: dict):
        self.id = uuid.uuid4().hex[:12]
        self.params = params
        self.status = "queued"
        self.title = ""
        self.scenes: list[dict] = []
        self.video_url = ""
        self.duration = 0.0
        self.error: Optional[str] = None
        self.events: list[tuple[str, dict]] = []
        self.queue: asyncio.Queue = asyncio.Queue()
        self.created = time.time()

    async def emit(self, event: str, data: dict) -> None:
        self.status = event
        self.events.append((event, data))
        await self.queue.put((event, data))

    def snapshot(self) -> dict:
        return {"id": self.id, "status": self.status, "title": self.title,
                "scenes": self.scenes, "video_url": self.video_url,
                "duration": round(self.duration, 2), "error": self.error,
                "params": self.params}


JOBS: dict[str, Job] = {}


class RenderRequest(BaseModel):
    topic: str = Field(..., min_length=4, max_length=MAX_TOPIC_LEN)
    voice: str = "nova"
    n_scenes: int = Field(7, ge=4, le=MAX_SCENES)
    music: bool = True


ACTIVE = 0


async def _worker(job: Job, api_key: Optional[str]) -> None:
    global ACTIVE
    ACTIVE += 1
    async def cb(event, data):
        if event == "script_done":
            job.title = data.get("title", "")
            job.scenes = data.get("scenes", [])
        elif event == "image_done":
            idx = data.get("index")
            if 0 <= idx < len(job.scenes):
                job.scenes[idx]["image_url"] = data.get("image_url")
        elif event == "done":
            job.video_url = data.get("video_url", "")
            job.duration = data.get("duration", 0.0)
        await job.emit(event, data)
    try:
        await job.emit("started", {"status": "Starting…"})
        provider = get_provider(api_key)
        await render_job(topic=job.params["topic"], voice=job.params["voice"],
                         n_scenes=job.params["n_scenes"], progress=cb, job_id=job.id,
                         music=job.params.get("music", True), provider=provider)
        await job.emit("complete", {"status": "Done"})
    except Exception as e:  # noqa: BLE001
        import traceback
        job.error = str(e)
        await job.emit("error", {"message": str(e), "trace": traceback.format_exc()[-600:]})
    finally:
        ACTIVE -= 1


@app.post("/api/render")
async def render(req: RenderRequest, x_api_key: Optional[str] = Header(default=None)):
    if ACTIVE >= MAX_ACTIVE_JOBS:
        raise HTTPException(429, f"Too many active jobs (max {MAX_ACTIVE_JOBS}). Try again shortly.")
    params = req.model_dump()
    if params["voice"] not in OPENAI_VOICES:
        params["voice"] = "nova"
    job = Job(params)
    JOBS[job.id] = job
    asyncio.create_task(_worker(job, x_api_key))
    return {"job_id": job.id, "snapshot": job.snapshot()}


@app.get("/api/voices")
async def voices():
    return {"voices": OPENAI_VOICES, "has_env_key": bool(__import__("os").getenv("OPENAI_API_KEY"))}


@app.get("/api/jobs/{job_id}")
async def job_status(job_id: str):
    job = JOBS.get(job_id)
    if not job:
        raise HTTPException(404, "job not found")
    return job.snapshot()


@app.get("/api/jobs/{job_id}/events")
async def job_events(job_id: str):
    job = JOBS.get(job_id)
    if not job:
        raise HTTPException(404, "job not found")

    async def gen():
        for ev, data in list(job.events):
            yield f"event: {ev}\ndata: {json.dumps(data)}\n\n"
        if job.status in ("complete", "error"):
            yield "event: close\ndata: {}\n\n"
            return
        while True:
            try:
                ev, data = await asyncio.wait_for(job.queue.get(), timeout=15.0)
            except asyncio.TimeoutError:
                yield ": keepalive\n\n"
                continue
            yield f"event: {ev}\ndata: {json.dumps(data)}\n\n"
            if ev in ("complete", "error"):
                yield "event: close\ndata: {}\n\n"
                return

    return StreamingResponse(gen(), media_type="text/event-stream",
                            headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"})


@app.get("/api/jobs/{job_id}/{name}")
async def job_file(job_id: str, name: str):
    job = JOBS.get(job_id)
    if not job:
        raise HTTPException(404, "job not found")
    safe = Path(name).name
    path = JOBS_DIR / job_id / safe
    if not path.exists() or not path.is_file():
        raise HTTPException(404, "file not found")
    if safe.endswith(".mp4"):
        return FileResponse(path, media_type="video/mp4", filename=safe)
    if safe.endswith(".png"):
        return FileResponse(path, media_type="image/png", filename=safe)
    raise HTTPException(400, "unsupported file type")


app.mount("/", StaticFiles(directory=str(PUBLIC), html=True), name="public")


if __name__ == "__main__":
    import uvicorn
    uvicorn.run("app.server:app", host="0.0.0.0", port=8000, log_level="info")
