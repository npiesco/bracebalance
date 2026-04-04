#!/usr/bin/env python3
"""Synthesize per-phase TTS narration placed at real timestamps, then mux with video."""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

DEMO_DIR = Path(__file__).resolve().parent
ROOT_DIR = DEMO_DIR.parents[1]
OUTPUT_DIR = ROOT_DIR / "demo" / "output"
TIMINGS_PATH = OUTPUT_DIR / "timings.json"
SAMPLE_RATE = 16000


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
        raise RuntimeError(f"Command failed: {' '.join(args)}\nstderr: {result.stderr[:500]}")
    return result


def get_duration(path: Path) -> float:
    result = run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(path)],
        timeout=20,
    )
    return float(result.stdout.strip())


def synthesize_segment(
    speechsdk: object,
    speech_config: object,
    text: str,
    out_path: Path,
) -> float:
    """Synthesize a single text segment to WAV, return its duration in seconds."""
    audio_config = speechsdk.audio.AudioOutputConfig(filename=str(out_path))  # type: ignore
    synthesizer = speechsdk.SpeechSynthesizer(  # type: ignore
        speech_config=speech_config, audio_config=audio_config,
    )
    result = synthesizer.speak_text_async(text).get()
    if result.reason != speechsdk.ResultReason.SynthesizingAudioCompleted:  # type: ignore
        raise RuntimeError(f"TTS failed: {result.reason}")
    return get_duration(out_path)


def synthesize_narration(video_duration: float, timings: dict[str, float]) -> Path:
    """Synthesize each narration segment, place at its timestamp, mix into one track."""
    import azure.cognitiveservices.speech as speechsdk

    speech_key = os.environ.get("AZURE_SPEECH_KEY") or os.environ.get("FOUNDRY_API_KEY")
    speech_region = os.environ.get("AZURE_SPEECH_REGION") or os.environ.get("FOUNDRY_REGION")
    speech_voice = os.environ.get("BRACEBALANCE_DEMO_VOICE", "en-US-AndrewMultilingualNeural")

    if not speech_key or not speech_region:
        raise RuntimeError("AZURE_SPEECH_KEY/FOUNDRY_API_KEY and AZURE_SPEECH_REGION/FOUNDRY_REGION required")

    from narration import SEGMENTS

    speech_config = speechsdk.SpeechConfig(subscription=speech_key, region=speech_region)
    speech_config.speech_synthesis_voice_name = speech_voice
    speech_config.set_speech_synthesis_output_format(
        speechsdk.SpeechSynthesisOutputFormat.Riff16Khz16BitMonoPcm,
    )

    # Synthesize each segment and compute its placement time
    segment_wavs: list[tuple[Path, float]] = []  # (wav_path, start_seconds)
    for seg in SEGMENTS:
        event = seg["after_event"]
        fallback_event = seg.get("fallback_event")
        fallback_offset = seg.get("fallback_offset", 0.0)

        if event in timings:
            start_at = timings[event] + seg["offset"]
        elif fallback_event and fallback_event in timings:
            start_at = timings[fallback_event] + fallback_offset
            print(f"  [tts] {seg['id']}: using fallback {fallback_event}+{fallback_offset}s")
        elif fallback_event is None and seg["offset"] > 0:
            start_at = seg["offset"]
        else:
            print(f"  [tts] skipping '{seg['id']}': no timing event available")
            continue

        start_at = max(0.0, min(start_at, video_duration - 5.0))
        raw_path = OUTPUT_DIR / f"_narr_{seg['id']}.wav"
        dur = synthesize_segment(speechsdk, speech_config, seg["text"], raw_path)
        print(f"  [tts] {seg['id']}: start={start_at:.1f}s  speech={dur:.1f}s")
        segment_wavs.append((raw_path, start_at))

    if not segment_wavs:
        raise RuntimeError("No narration segments were synthesized")

    # Build ffmpeg filter: delay each segment, then amix all together, pad to video length
    inputs: list[str] = []
    filter_parts: list[str] = []
    for i, (wav_path, start_at) in enumerate(segment_wavs):
        inputs.extend(["-i", str(wav_path)])
        delay_ms = int(start_at * 1000)
        filter_parts.append(f"[{i}]adelay={delay_ms}|{delay_ms}[d{i}]")

    mix_inputs = "".join(f"[d{i}]" for i in range(len(segment_wavs)))
    filter_parts.append(
        f"{mix_inputs}amix=inputs={len(segment_wavs)}:duration=longest:normalize=0,"
        f"apad=whole_dur={video_duration:.3f}"
    )
    filter_graph = ";".join(filter_parts)

    narration_path = OUTPUT_DIR / "narration.wav"
    run(
        [
            "ffmpeg", "-y",
            *inputs,
            "-filter_complex", filter_graph,
            "-ar", str(SAMPLE_RATE),
            "-ac", "1",
            str(narration_path),
        ],
        timeout=60,
    )

    # Clean up intermediate WAVs
    for wav_path, _ in segment_wavs:
        wav_path.unlink(missing_ok=True)

    nar_dur = get_duration(narration_path)
    print(f"  [tts] narration track: {nar_dur:.1f}s (video: {video_duration:.1f}s)")
    return narration_path


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

    # Validate audio stream exists
    audio_check = run(
        ["ffprobe", "-v", "error", "-select_streams", "a",
         "-show_entries", "stream=codec_type", "-of", "csv=p=0", str(final_path)],
        timeout=20,
    )
    if not audio_check.stdout.strip():
        raise RuntimeError(f"Final video has no audio stream: {final_path}")

    size_mb = final_path.stat().st_size / (1024 * 1024)
    print(f"[mux] final video: {final_path} ({size_mb:.1f} MB)")


def main() -> int:
    load_env()

    recording_path = Path(os.environ.get("DEMO_RECORDING_PATH", str(OUTPUT_DIR / "bracebalance_demo.mp4")))
    final_path = OUTPUT_DIR / "bracebalance_demo_final.mp4"

    if not recording_path.is_file():
        print(f"Recording not found: {recording_path}", file=sys.stderr)
        return 1

    if not TIMINGS_PATH.is_file():
        print(f"Timings not found: {TIMINGS_PATH}", file=sys.stderr)
        return 1

    timings = json.loads(TIMINGS_PATH.read_text())
    print(f"[tts] loaded timings: {timings}")

    video_duration = get_duration(recording_path)
    print(f"[tts] video duration: {video_duration:.1f}s")

    narration_path = synthesize_narration(video_duration, timings)
    merge_final_video(recording_path, narration_path, final_path)

    print(f"\nDone. Final video: {final_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
