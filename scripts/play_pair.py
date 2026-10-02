#!/usr/bin/env python3
"""Preflight exactly two local PCM clips, then play once each with one fixed gap.

No gain, default-device, Bluetooth, or host-audio changes. Explicit invocation is
required. All decoding/header checks and sink validation finish before playback.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import shutil
import subprocess
import time
import wave


def prepare(paths):
    if len(paths) != 2:
        raise ValueError('Exactly two clips are required')
    clips = []
    for item in paths:
        path = Path(item).resolve(strict=True)
        stat = path.stat()
        digest = hashlib.sha256()
        with wave.open(str(path), 'rb') as reader:
            params = reader.getparams()
            if params.nchannels not in (1, 2) or params.sampwidth not in (2, 3, 4):
                raise ValueError('Expected mono/stereo 16/24/32-bit PCM')
            size = 0
            while data := reader.readframes(65536):
                size += len(data)
                digest.update(data)
            if not params.nframes or size != params.nframes * params.nchannels * params.sampwidth:
                raise ValueError('Empty or truncated clip')
        if (stat.st_size, stat.st_mtime_ns) != (path.stat().st_size, path.stat().st_mtime_ns):
            raise ValueError('Clip changed during preflight')
        clips.append(dict(path=str(path), seconds=params.nframes / params.framerate,
                          rate=params.framerate, pcm_sha256=digest.hexdigest()))
    if abs(clips[0]['seconds'] - clips[1]['seconds']) > max(1 / x['rate'] for x in clips):
        raise ValueError('Comparison clips must cover equal durations')
    if clips[0]['pcm_sha256'] == clips[1]['pcm_sha256'] and clips[0]['rate'] == clips[1]['rate']:
        raise ValueError('Clips contain identical PCM; refusing a duplicate comparison')
    return clips


def check_sink(target):
    if not shutil.which('pw-play'):
        raise RuntimeError('pw-play is unavailable')
    nodes = json.loads(subprocess.check_output(['pw-dump'], text=True, timeout=5))
    found = any(n.get('info', {}).get('props', {}).get('node.name') == target
                and n.get('info', {}).get('props', {}).get('media.class') == 'Audio/Sink'
                for n in nodes)
    if not found:
        raise RuntimeError('Requested audio sink is not present; no fallback playback')


def play(clip, target):
    subprocess.run(['pw-play', '--target', target, clip['path']], check=True,
                   timeout=clip['seconds'] + 10)


def run_pair(paths, target, gap=1.0, *, sink_check=check_sink, player=play, pause=time.sleep):
    if not math.isfinite(gap) or not 0 <= gap <= 5:
        raise ValueError('Gap must be between zero and five seconds')
    clips = prepare(paths)
    sink_check(target)
    # No decoding, media preparation, discovery or assistant round-trips below.
    started = time.monotonic()
    player(clips[0], target)
    pause(gap)
    player(clips[1], target)
    return dict(clips=clips, target=target, requested_gap_seconds=gap,
                elapsed_seconds=time.monotonic() - started, plays_per_clip=1,
                host_audio_changes=False, gain_changes=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('clips', type=Path, nargs=2)
    parser.add_argument('--target', required=True, help='Explicit PipeWire sink node.name')
    parser.add_argument('--gap', type=float, default=1.0)
    args = parser.parse_args()
    print(json.dumps(run_pair(args.clips, args.target, args.gap), indent=2))


if __name__ == '__main__':
    main()
