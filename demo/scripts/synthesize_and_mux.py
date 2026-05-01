#!/usr/bin/env python3
"""Make final demo video: title card, sped-up content, narration fit to video, outro, concat."""
from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

DEMO_DIR = Path(__file__).resolve().parent
ROOT_DIR = DEMO_DIR.parents[1]
OUTPUT_DIR = ROOT_DIR / "demo" / "output"
LOGO_PATH = ROOT_DIR / "bracebalance-logo.png"
FONT_BOLD = "/usr/share/fonts/noto/NotoSans-Bold.ttf"
FONT_REG = "/usr/share/fonts/noto/NotoSans-Regular.ttf"

# Speed zones: (original_start, original_end, factor). 0 = to source end.
SPEED_ZONES: list[tuple[float, float, float]] = [
    (0.0, 70.0, 3.0),      # typing + trust prompt
    (70.0, 130.0, 1.0),    # Copilot working + summary (full speed)
    (130.0, 0.0, 6.0),     # post-completion idle → fast-forward to end
]

TITLE_DURATION = 6.0   # seconds for intro/outro cards


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


def build_atempo_chain(speedup: float) -> str:
    """Split large tempo changes into ffmpeg-supported atempo stages."""
    factors: list[float] = []
    remaining = speedup

    while remaining > 2.0:
        factors.append(2.0)
        remaining /= 2.0

    while remaining < 0.5:
        factors.append(0.5)
        remaining /= 0.5

    factors.append(remaining)
    return ",".join(f"atempo={factor:.4f}" for factor in factors)


# ── Title cards ──────────────────────────────────────────────────────────────

def generate_title_card(output_path: Path, duration: float, title: str, subtitle: str, w: int, h: int) -> None:
    logo_filter = ""
    inputs: list[str] = ["-f", "lavfi", "-i", f"color=c=0x1a1a2e:s={w}x{h}:r=30:d={duration}"]

    if LOGO_PATH.exists():
        inputs += ["-i", str(LOGO_PATH)]
        logo_scale = min(160, h // 3)
        logo_y = h // 2 - logo_scale - 20
        text_y = h // 2 + 20
        sub_y = h // 2 + 80
        logo_filter = (
            f"[1:v]scale={logo_scale}:{logo_scale}[logo];"
            f"[0:v][logo]overlay=(W-w)/2:{logo_y}[withlogo];"
            f"[withlogo]"
        )
        base = "[withlogo]"
    else:
        text_y = h // 2 - 30
        sub_y = h // 2 + 30
        base = "[0:v]"
        logo_filter = f"{base}"

    text_filter = (
        f"{logo_filter}"
        f"drawtext=text='{title}':fontfile={FONT_BOLD}:fontsize=52:fontcolor=white:"
        f"x=(w-text_w)/2:y={text_y},"
        f"drawtext=text='{subtitle}':fontfile={FONT_REG}:fontsize=22:fontcolor=0xaaaaaa:"
        f"x=(w-text_w)/2:y={sub_y}[out]"
    )

    run(
        [
            "ffmpeg", "-y",
            *inputs,
            "-filter_complex", text_filter,
            "-map", "[out]",
            "-c:v", "libx264", "-crf", "18", "-r", "30",
            "-pix_fmt", "yuv420p",
            "-an",
            str(output_path),
        ],
        timeout=60,
    )
    print(f"[title] {output_path.name}: {duration:.0f}s")


# ── Step 1: Create sped-up silent video (full length) ────────────────────────

def create_speedup_video(
    src: Path, dst: Path, zones: list[tuple[float, float, float]],
) -> float:
    src_dur = get_duration(src)
    input_args: list[str] = []
    filter_parts: list[str] = []

    for i, (zs, ze, zf) in enumerate(zones):
        actual_end = ze if ze > 0 else src_dur
        args: list[str] = []
        if zs > 0:
            args.extend(["-ss", f"{zs:.3f}"])
        clip_dur = actual_end - zs
        if clip_dur > 0:
            args.extend(["-t", f"{clip_dur:.3f}"])
        args.extend(["-i", str(src)])
        input_args.extend(args)

        if zf == 1.0:
            filter_parts.append(f"[{i}:v]setpts=PTS-STARTPTS[z{i}]")
        else:
            filter_parts.append(f"[{i}:v]setpts=(PTS-STARTPTS)/{zf}[z{i}]")

    zone_refs = "".join(f"[z{i}]" for i in range(len(zones)))
    filter_parts.append(f"{zone_refs}concat=n={len(zones)}:v=1:a=0[outv]")

    zone_desc = ", ".join(
        f"{zs:.0f}-{'end' if ze == 0 else f'{ze:.0f}'}s@{zf:.0f}x"
        for zs, ze, zf in zones
    )
    print(f"[video] speed zones: {zone_desc} (source: {src_dur:.0f}s)")
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


# ── Step 2: Synthesize narration, then fit to video duration ─────────────────

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


def synthesize_narration_fit(target_seconds: float, out_path: Path) -> float:
    """Synthesize NARRATION_TEXT, then use atempo to fit it to target_seconds."""
    from narration import NARRATION_TEXT

    speechsdk, speech_config = _init_speech()

    raw_path = OUTPUT_DIR / "_narration_raw.wav"
    audio_config = speechsdk.audio.AudioOutputConfig(filename=str(raw_path))
    synthesizer = speechsdk.SpeechSynthesizer(speech_config=speech_config, audio_config=audio_config)
    result = synthesizer.speak_text_async(NARRATION_TEXT).get()
    if result.reason != speechsdk.ResultReason.SynthesizingAudioCompleted:
        raise RuntimeError(f"TTS failed: {result.reason}")

    raw_dur = get_duration(raw_path)
    print(f"[audio] synthesized: {raw_dur:.1f}s, target: {target_seconds:.1f}s")

    # Fit: speed up if longer than target, pad with silence if shorter
    if raw_dur > target_seconds + 0.5:
        tempo = raw_dur / target_seconds
        af_filter = f"{build_atempo_chain(tempo)},apad=whole_dur={target_seconds:.3f}"
        print(f"[audio] speeding up {tempo:.2f}x to fit")
    else:
        af_filter = f"apad=whole_dur={target_seconds:.3f}"
        print(f"[audio] padding to fill {target_seconds:.1f}s")

    run(
        [
            "ffmpeg", "-y",
            "-i", str(raw_path),
            "-af", af_filter,
            "-ar", "16000", "-ac", "1",
            str(out_path),
        ],
        timeout=60,
    )
    raw_path.unlink(missing_ok=True)
    dur = get_duration(out_path)
    print(f"[audio] narration: {dur:.1f}s")
    return dur


# ── Step 3: Mux main video + audio ──────────────────────────────────────────

def mux_with_audio(video_path: Path, audio_path: Path, out_path: Path) -> None:
    video_dur = get_duration(video_path)
    audio_dur = get_duration(audio_path)
    pad_dur = max(0.0, audio_dur - video_dur + 0.05)

    if pad_dur > 0:
        print(f"[mux] padding video by {pad_dur:.2f}s to preserve full narration")
        run(
            [
                "ffmpeg", "-y",
                "-i", str(video_path),
                "-i", str(audio_path),
                "-filter_complex", f"[0:v]tpad=stop_mode=clone:stop_duration={pad_dur:.3f}[vout]",
                "-map", "[vout]", "-map", "1:a:0",
                "-c:v", "libx264", "-preset", "fast", "-crf", "18",
                "-pix_fmt", "yuv420p",
                "-c:a", "aac", "-b:a", "192k",
                "-movflags", "+faststart",
                str(out_path),
            ],
            timeout=300,
        )
        return

    run(
        [
            "ffmpeg", "-y",
            "-i", str(video_path),
            "-i", str(audio_path),
            "-c:v", "copy",
            "-c:a", "aac", "-b:a", "192k",
            "-map", "0:v:0", "-map", "1:a:0",
            "-movflags", "+faststart",
            str(out_path),
        ],
        timeout=300,
    )


# ── Step 4: Concat title + main + outro ─────────────────────────────────────

def concat_segments(segments: list[Path], out_path: Path, w: int, h: int) -> None:
    n = len(segments)
    inputs: list[str] = []
    filter_parts: list[str] = []

    for i, seg in enumerate(segments):
        inputs.extend(["-i", str(seg)])
        filter_parts.append(
            f"[{i}:v:0]scale={w}:{h}:force_original_aspect_ratio=decrease,"
            f"pad={w}:{h}:(ow-iw)/2:(oh-ih)/2[v{i}];"
        )

    # Add silent audio for video-only segments (title cards have no audio track)
    for i, seg in enumerate(segments):
        probe = subprocess.run(
            ["ffprobe", "-v", "error", "-select_streams", "a",
             "-show_entries", "stream=codec_type", "-of", "csv=p=0", str(seg)],
            capture_output=True, text=True, timeout=10,
        )
        if probe.stdout.strip():
            filter_parts.append(f"[{i}:a:0]aformat=sample_rates=44100:channel_layouts=mono[a{i}];")
        else:
            dur = get_duration(seg)
            filter_parts.append(
                f"aevalsrc=0:s=44100:c=mono:d={dur:.3f}[a{i}];"
            )

    v_refs = "".join(f"[v{i}]" for i in range(n))
    a_refs = "".join(f"[a{i}]" for i in range(n))
    filter_parts.append(f"{v_refs}{a_refs}concat=n={n}:v=1:a=1[outv][outa]")

    filter_str = "".join(filter_parts)

    run(
        [
            "ffmpeg", "-y",
            *inputs,
            "-filter_complex", filter_str,
            "-map", "[outv]", "-map", "[outa]",
            "-c:v", "libx264", "-crf", "18", "-r", "30",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac", "-b:a", "192k",
            "-movflags", "+faststart",
            str(out_path),
        ],
        timeout=600,
    )
    size_mb = out_path.stat().st_size / (1024 * 1024)
    print(f"[concat] final: {get_duration(out_path):.0f}s, {size_mb:.1f} MB → {out_path.name}")


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> int:
    load_env()

    recording_path = Path(os.environ.get("DEMO_RECORDING_PATH", str(OUTPUT_DIR / "bracebalance_demo.mp4")))
    final_path = OUTPUT_DIR / "bracebalance_demo_final.mp4"

    if not recording_path.is_file():
        print(f"Recording not found: {recording_path}", file=sys.stderr)
        return 1

    # Probe source resolution
    probe = run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0",
         "-show_entries", "stream=width,height", "-of", "csv=p=0", str(recording_path)],
        timeout=15,
    )
    w_str, h_str = probe.stdout.strip().split(",")
    w, h = int(w_str), int(h_str)
    print(f"[video] source resolution: {w}x{h}")

    # Step 1: sped-up silent video
    speedup_path = OUTPUT_DIR / "_speedup.mp4"
    video_dur = create_speedup_video(recording_path, speedup_path, SPEED_ZONES)

    # Step 2: synthesize narration fit to video
    narration_path = OUTPUT_DIR / "_narration.wav"
    synthesize_narration_fit(video_dur, narration_path)

    # Step 3: mux main content with audio
    main_path = OUTPUT_DIR / "_main.mp4"
    mux_with_audio(speedup_path, narration_path, main_path)

    # Step 4: title cards
    intro_path = OUTPUT_DIR / "_intro.mp4"
    outro_path = OUTPUT_DIR / "_outro.mp4"
    generate_title_card(
        intro_path, TITLE_DURATION,
        "BraceBalance",
        "structural delimiter checking for humans and agents",
        w, h,
    )
    generate_title_card(
        outro_path, TITLE_DURATION,
        "BraceBalance",
        "CLI  .  MCP  .  Rust library  --  github.com/npiesco/bracebalance",
        w, h,
    )

    # Step 5: concat intro + main + outro
    concat_segments([intro_path, main_path, outro_path], final_path, w, h)

    # Cleanup temps
    for p in [speedup_path, narration_path, main_path, intro_path, outro_path]:
        p.unlink(missing_ok=True)

    final_dur = get_duration(final_path)
    final_size = final_path.stat().st_size
    print(f"\n[DONE] {final_dur:.0f}s, {final_size / (1024*1024):.1f} MB")
    print(f"Final video: {final_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
