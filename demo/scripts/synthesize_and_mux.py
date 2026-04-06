#!/usr/bin/env python3
"""Make final demo video: speed-up raw recording, synthesize narration sized to video length, mux."""
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
RECORDING_START_FILE = OUTPUT_DIR / "recording_started_at"

# Speed zones: (original_start, original_end, factor). 0 = to source end.
SPEED_ZONES: list[tuple[float, float, float]] = [
    (0.0, 70.0, 3.0),      # typing + trust prompt
    (70.0, 130.0, 1.0),    # Copilot working + summary (full speed)
    (130.0, 218.0, 6.0),   # post-completion idle → fast-forward to end
]


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
        timeout=30,
    )
    return float(result.stdout.strip())


# ── Step 1: Create sped-up silent video (full length, no trimming) ───────────

def create_speedup_video(
    src: Path, dst: Path, zones: list[tuple[float, float, float]],
) -> float:
    input_args: list[str] = []
    filter_parts: list[str] = []

    for i, (zs, ze, zf) in enumerate(zones):
        args: list[str] = []
        if zs > 0:
            args.extend(["-ss", f"{zs:.3f}"])
        if ze > 0:
            args.extend(["-t", f"{ze - zs:.3f}"])
        args.extend(["-i", str(src)])
        input_args.extend(args)

        if zf == 1.0:
            filter_parts.append(f"[{i}:v]setpts=PTS-STARTPTS[z{i}]")
        else:
            filter_parts.append(f"[{i}:v]setpts=(PTS-STARTPTS)/{zf}[z{i}]")

    zone_refs = "".join(f"[z{i}]" for i in range(len(zones)))
    filter_parts.append(f"{zone_refs}concat=n={len(zones)}:v=1:a=0[outv]")

    zone_desc = ", ".join(
        f"{zs:.0f}-{ze:.0f}s@{zf:.0f}x" if ze > 0 else f"{zs:.0f}s+@{zf:.0f}x"
        for zs, ze, zf in zones
    )
    print(f"[video] speed zones: {zone_desc}")
    run(
        [
            "ffmpeg", "-y",
            *input_args,
            "-filter_complex", ";".join(filter_parts),
            "-map", "[outv]",
            "-c:v", "libx264", "-preset", "fast", "-crf", "18",
            "-tune", "stillimage", "-pix_fmt", "yuv420p",
            "-an",
            str(dst),
        ],
        timeout=600,
    )
    dur = get_duration(dst)
    print(f"[video] sped-up video: {dur:.1f}s")
    return dur


# ── Step 2: Measure voice tempo, size text to video, synthesize ──────────────

def _init_speech():
    import azure.cognitiveservices.speech as speechsdk

    speech_key = os.environ.get("AZURE_SPEECH_KEY") or os.environ.get("FOUNDRY_API_KEY")
    speech_region = os.environ.get("AZURE_SPEECH_REGION") or os.environ.get("FOUNDRY_REGION")
    speech_voice = os.environ.get("BRACEBALANCE_DEMO_VOICE", "en-US-AndrewMultilingualNeural")

    if not speech_key or not speech_region:
        raise RuntimeError("AZURE_SPEECH_KEY/FOUNDRY_API_KEY and AZURE_SPEECH_REGION/FOUNDRY_REGION required")

    speech_config = speechsdk.SpeechConfig(subscription=speech_key, region=speech_region)
    speech_config.speech_synthesis_voice_name = speech_voice
    speech_config.set_speech_synthesis_output_format(
        speechsdk.SpeechSynthesisOutputFormat.Riff16Khz16BitMonoPcm,
    )
    return speechsdk, speech_config


def _synthesize(speechsdk, speech_config, text: str, out_path: Path) -> float:
    audio_config = speechsdk.audio.AudioOutputConfig(filename=str(out_path))
    synthesizer = speechsdk.SpeechSynthesizer(
        speech_config=speech_config, audio_config=audio_config,
    )
    result = synthesizer.speak_text_async(text).get()
    if result.reason != speechsdk.ResultReason.SynthesizingAudioCompleted:
        raise RuntimeError(f"TTS failed: {result.reason}")
    return get_duration(out_path)


def synthesize_narration_to_length(target_seconds: float, out_path: Path) -> float:
    from narration import NARRATION_TEXT

    speechsdk, speech_config = _init_speech()

    # Calibrate: synthesize the full text, measure words/sec
    cal_path = OUTPUT_DIR / "_cal.wav"
    cal_dur = _synthesize(speechsdk, speech_config, NARRATION_TEXT, cal_path)
    cal_path.unlink(missing_ok=True)

    words = NARRATION_TEXT.split()
    wps = len(words) / cal_dur
    print(f"[audio] calibration: {len(words)} words in {cal_dur:.1f}s = {wps:.2f} words/sec")

    # Calculate how many words we need for the target duration
    target_words = int(target_seconds * wps)
    print(f"[audio] target: {target_seconds:.1f}s × {wps:.2f} wps = {target_words} words")

    # If we need more words than we have, repeat the text to fill
    if target_words <= len(words):
        final_text = " ".join(words[:target_words])
    else:
        repeats = (target_words // len(words)) + 1
        all_words = (words * repeats)[:target_words]
        final_text = " ".join(all_words)

    print(f"[audio] final narration: {len(final_text.split())} words")
    dur = _synthesize(speechsdk, speech_config, final_text, out_path)
    print(f"[audio] narration: {dur:.1f}s (target was {target_seconds:.1f}s)")
    return dur


# ── Step 3: Mux video + audio (no trimming) ─────────────────────────────────

def mux_final(video_path: Path, audio_path: Path, final_path: Path) -> None:
    print(f"[mux] video + audio -> {final_path.name}")
    run(
        [
            "ffmpeg", "-y",
            "-i", str(video_path),
            "-i", str(audio_path),
            "-c:v", "libx264", "-preset", "medium", "-crf", "18",
            "-tune", "stillimage", "-pix_fmt", "yuv420p",
            "-c:a", "aac", "-b:a", "192k",
            "-map", "0:v:0", "-map", "1:a:0",
            "-shortest",
            "-movflags", "+faststart",
            str(final_path),
        ],
        timeout=600,
    )

    audio_check = run(
        ["ffprobe", "-v", "error", "-select_streams", "a",
         "-show_entries", "stream=codec_type", "-of", "csv=p=0", str(final_path)],
        timeout=20,
    )
    if not audio_check.stdout.strip():
        raise RuntimeError(f"Final video has no audio stream: {final_path}")

    size_mb = final_path.stat().st_size / (1024 * 1024)
    print(f"[mux] final: {final_path} ({size_mb:.1f} MB)")


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> int:
    load_env()

    recording_path = Path(os.environ.get("DEMO_RECORDING_PATH", str(OUTPUT_DIR / "bracebalance_demo.mp4")))
    final_path = OUTPUT_DIR / "bracebalance_demo_final.mp4"

    if not recording_path.is_file():
        print(f"Recording not found: {recording_path}", file=sys.stderr)
        return 1

    # Step 1: sped-up video (full length)
    speedup_path = OUTPUT_DIR / "_speedup.mp4"
    video_dur = create_speedup_video(recording_path, speedup_path, SPEED_ZONES)

    # Step 2: narration sized to fill the video
    narration_path = OUTPUT_DIR / "_narration.wav"
    narration_dur = synthesize_narration_to_length(video_dur, narration_path)

    # Step 3: mux — no trimming
    mux_final(speedup_path, narration_path, final_path)

    # Clean up
    speedup_path.unlink(missing_ok=True)
    narration_path.unlink(missing_ok=True)

    final_dur = get_duration(final_path)
    final_size = final_path.stat().st_size
    print(f"\n[DONE] {final_dur:.0f}s, {final_size / (1024*1024):.1f} MB")
    print(f"Final video: {final_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
