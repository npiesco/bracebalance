#!/usr/bin/env python3
"""Synthesize TTS narration and mux it with the demo recording."""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

DEMO_DIR = Path(__file__).resolve().parent
ROOT_DIR = DEMO_DIR.parents[1]
OUTPUT_DIR = ROOT_DIR / "demo" / "output"


def load_env() -> None:
    env_path = DEMO_DIR / ".env"
    if not env_path.exists():
        return
    for raw in env_path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        os.environ.setdefault(key.strip(), value.strip())


def run(args: list[str], timeout: int = 120) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    if result.returncode != 0:
        raise RuntimeError(f"Command failed: {' '.join(args)}\n{result.stderr}")
    return result


def get_duration(path: Path) -> float:
    result = run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(path)],
        timeout=20,
    )
    return float(result.stdout.strip())


def synthesize_narration(recording_path: Path) -> Path:
    import azure.cognitiveservices.speech as speechsdk

    speech_key = os.environ.get("AZURE_SPEECH_KEY") or os.environ.get("FOUNDRY_API_KEY")
    speech_region = os.environ.get("AZURE_SPEECH_REGION") or os.environ.get("FOUNDRY_REGION")
    speech_voice = os.environ.get("BRACEBALANCE_DEMO_VOICE", "en-US-AndrewMultilingualNeural")

    if not speech_key or not speech_region:
        raise RuntimeError("AZURE_SPEECH_KEY/FOUNDRY_API_KEY and AZURE_SPEECH_REGION/FOUNDRY_REGION must be set")

    from narration import NARRATION

    speech_config = speechsdk.SpeechConfig(subscription=speech_key, region=speech_region)
    speech_config.speech_synthesis_voice_name = speech_voice
    speech_config.set_speech_synthesis_output_format(
        speechsdk.SpeechSynthesisOutputFormat.Riff16Khz16BitMonoPcm,
    )

    raw_path = OUTPUT_DIR / "narration_raw.wav"
    padded_path = OUTPUT_DIR / "narration.wav"
    audio_config = speechsdk.audio.AudioOutputConfig(filename=str(raw_path))
    synthesizer = speechsdk.SpeechSynthesizer(speech_config=speech_config, audio_config=audio_config)

    print(f"[tts] synthesizing narration ({len(NARRATION)} chars)...")
    result = synthesizer.speak_text_async(NARRATION).get()
    if result.reason != speechsdk.ResultReason.SynthesizingAudioCompleted:
        raise RuntimeError(f"Speech synthesis failed: {result.reason}")

    speech_duration = get_duration(raw_path)
    video_duration = get_duration(recording_path)
    print(f"[tts] speech={speech_duration:.1f}s  video={video_duration:.1f}s")

    # Fit narration to video: speed up if too long, pad with silence if too short
    if speech_duration > video_duration + 0.5:
        tempo = min(2.0, speech_duration / video_duration)
        af_filter = f"atempo={tempo:.4f},apad=whole_dur={video_duration:.3f}"
        print(f"[tts] fitting: tempo={tempo:.2f}x")
    else:
        af_filter = f"apad=whole_dur={video_duration:.3f}"
        print("[tts] fitting: pad with silence")

    run(
        [
            "ffmpeg", "-y",
            "-i", str(raw_path),
            "-af", af_filter,
            "-ar", "16000",
            "-ac", "1",
            str(padded_path),
        ],
        timeout=60,
    )
    return padded_path


def merge_final_video(recording_path: Path, narration_path: Path, final_path: Path) -> None:
    print(f"[mux] merging video + narration -> {final_path}")
    run(
        [
            "ffmpeg", "-y",
            "-i", str(recording_path),
            "-i", str(narration_path),
            "-c:v", "libx264",
            "-preset", "medium",
            "-crf", "18",
            "-tune", "stillimage",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac",
            "-b:a", "192k",
            "-map", "0:v:0",
            "-map", "1:a:0",
            "-shortest",
            "-movflags", "+faststart",
            str(final_path),
        ],
        timeout=600,
    )
    size_mb = final_path.stat().st_size / (1024 * 1024)
    print(f"[mux] final video: {final_path} ({size_mb:.1f} MB)")


def main() -> int:
    load_env()

    recording_path = Path(os.environ.get("DEMO_RECORDING_PATH", str(OUTPUT_DIR / "bracebalance_demo.mp4")))
    final_path = OUTPUT_DIR / "bracebalance_demo_final.mp4"

    if not recording_path.is_file():
        print(f"Recording not found: {recording_path}", file=sys.stderr)
        return 1

    narration_path = synthesize_narration(recording_path)
    merge_final_video(recording_path, narration_path, final_path)

    print(f"\nDone. Final video: {final_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
