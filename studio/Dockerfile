# Build with ffmpeg baked in.
FROM python:3.12-slim

# ffmpeg is the only system dependency (image/audio/video assembly).
RUN apt-get update && apt-get install -y --no-install-recommends ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY app ./app
# jobs are written here; mount a volume in production to persist renders
RUN mkdir -p jobs
ENV PYTHONUNBUFFERED=1

EXPOSE 8000
CMD ["uvicorn", "app.server:app", "--host", "0.0.0.0", "--port", "8000"]
