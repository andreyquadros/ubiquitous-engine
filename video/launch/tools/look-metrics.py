#!/usr/bin/env python3
"""
Per-scene brightness metrics for the v2 look (brief/v2-look.md §2).

Every frame is reduced to a 192x108 grey (luma, 0-1) image (video: ffmpeg
`scale=192:108,format=gray`, the brief's measurement; stills: Rec.709 luma of a
bilinear 192x108 downscale, which reads ~0.005 LOWER than the same frame after
the H.264 encode, so stills are the conservative side); per scene we report
mean, p10, p50, p90 and the fraction of pixels brighter than 0.3, then check the
§2 brightness targets:

  scene mean >= 0.14 (s04: >= 0.05), film mean >= 0.16,
  frac > 0.3 >= 0.08 (not s04), p10 <= 0.12.

Usage (from video/launch/):
  python3 tools/look-metrics.py out/launch-v1-master.mp4
  python3 tools/look-metrics.py out/review/v2/stills            # <Comp>-<frame>.png|jpg
  python3 tools/look-metrics.py <input> --phone out/review/v2/phone   # + 480x270 downscales
  python3 tools/look-metrics.py <input> --step 2 --json out.json

Stills: the frame number is the trailing integer of the file name; frames of a
scene composition (S07-0030) or a group (G3-0059) are mapped to film frames with
the storyboard / timeline offsets. For a video, every `--step`-th frame is used
(default 1) and the film mean is the mean over all sampled frames.
Phone downscales of a video are written for the middle frame of each scene
(plus the frames given with --phone-frames).
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
FFMPEG = os.environ.get('FFMPEG', '/usr/local/bin/ffmpeg')
if not os.path.exists(FFMPEG):
    FFMPEG = 'ffmpeg'
GW, GH = 192, 108
PW, PH = 480, 270
GROUP_START = {'G1': 0, 'G2': 240, 'G3': 450, 'G4': 660, 'G5': 930, 'G6': 1230}

TARGET_MEAN = 0.14
TARGET_FILM = 0.16
TARGET_S04 = 0.05
TARGET_BRIGHT = 0.08
TARGET_P10 = 0.12


def load_scenes():
    with open(os.path.join(ROOT, 'brief', 'storyboard.json')) as fh:
        sb = json.load(fh)
    return [(s['id'], s['startFrame'], s['durationInFrames']) for s in sb['scenes']], sb.get('fps', 30)


def scene_of(frame, scenes):
    for sid, start, dur in scenes:
        if start <= frame < start + dur:
            return sid
    return None


def luma(rgb: np.ndarray) -> np.ndarray:
    """Rec.709 luma of an 8-bit RGB array (gamma-encoded, like a grey downscale)."""
    f = rgb.astype(np.float32) / 255.0
    return 0.2126 * f[..., 0] + 0.7152 * f[..., 1] + 0.0722 * f[..., 2]


def grey_small(img: Image.Image) -> np.ndarray:
    return luma(np.asarray(img.convert('RGB').resize((GW, GH), Image.BILINEAR)))


def iter_video(path, step):
    """Yield (frame, grey 192x108) from an mp4: ffmpeg `scale=192:108,format=gray` (the brief's
    measurement: the encoded Y plane, expanded to full range)."""
    cmd = [FFMPEG, '-v', 'error', '-i', path, '-vf', f'scale={GW}:{GH},format=gray', '-f', 'rawvideo', '-pix_fmt', 'gray', '-']
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE)
    size = GW * GH
    i = 0
    while True:
        buf = proc.stdout.read(size)
        if len(buf) < size:
            break
        if i % step == 0:
            yield i, np.frombuffer(buf, np.uint8).reshape(GH, GW).astype(np.float32) / 255.0
        i += 1
    proc.wait()


def video_phone(path, frames, outdir):
    """480x270 downscales of the given film frames of a video."""
    if not frames:
        return
    sel = '+'.join(f'eq(n\\,{f})' for f in sorted(frames))
    pattern = os.path.join(outdir, 'phone-tmp-%04d.png')
    subprocess.run([FFMPEG, '-v', 'error', '-y', '-i', path, '-vf', f'select={sel},scale={PW}:{PH}:flags=lanczos',
                    '-fps_mode', 'passthrough', pattern], check=True)
    for i, f in enumerate(sorted(frames), start=1):
        src = os.path.join(outdir, f'phone-tmp-{i:04d}.png')
        if os.path.exists(src):
            os.replace(src, os.path.join(outdir, f'phone-{f:04d}.png'))


def frame_from_name(name, scenes):
    m = re.match(r'^(?P<comp>[A-Za-z0-9]+?)-(?P<f>\d+)\.(png|jpe?g)$', name)
    if not m:
        return None
    comp, f = m.group('comp'), int(m.group('f'))
    if comp in GROUP_START:
        return GROUP_START[comp] + f
    sm = re.match(r'^S(\d\d)$', comp)
    if sm:
        idx = int(sm.group(1)) - 1
        if 0 <= idx < len(scenes):
            return scenes[idx][1] + f
    return f  # Launch or anything else: film frames


def iter_stills(d, scenes, want_phone):
    items = []
    for name in os.listdir(d):
        fr = frame_from_name(name, scenes)
        if fr is not None:
            items.append((fr, name))
    for fr, name in sorted(items):
        img = Image.open(os.path.join(d, name)).convert('RGB')
        phone = img.resize((PW, PH), Image.LANCZOS) if want_phone else None
        yield fr, grey_small(img), phone, name


def stats(values: np.ndarray):
    return {
        'mean': float(values.mean()),
        'p10': float(np.percentile(values, 10)),
        'p50': float(np.percentile(values, 50)),
        'p90': float(np.percentile(values, 90)),
        'bright': float((values > 0.3).mean()),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('input', help='mp4 file or directory of <Comp>-<frame>.png|jpg stills')
    ap.add_argument('--phone', metavar='OUTDIR', help='write 480x270 downscales here')
    ap.add_argument('--phone-frames', default='', help='video only: extra film frames to write, comma list')
    ap.add_argument('--step', type=int, default=1, help='video only: sample every Nth frame (default 1)')
    ap.add_argument('--json', metavar='FILE', help='also write the per-scene numbers as JSON')
    ap.add_argument('--quiet', action='store_true', help='table only, no per-still lines')
    args = ap.parse_args()

    scenes, _fps = load_scenes()
    per_scene: dict[str, list[np.ndarray]] = {s[0]: [] for s in scenes}
    per_frame = []
    if args.phone:
        os.makedirs(args.phone, exist_ok=True)

    if os.path.isdir(args.input):
        for fr, g, phone, name in iter_stills(args.input, scenes, bool(args.phone)):
            sid = scene_of(fr, scenes)
            if sid is None:
                continue
            per_scene[sid].append(g)
            st = stats(g)
            per_frame.append((fr, sid, st))
            if not args.quiet:
                print(f'  {name:<22} f{fr:>5}  {sid:<26} mean {st["mean"]:.3f}  p10 {st["p10"]:.3f}  >0.3 {st["bright"]:.3f}')
            if phone is not None:
                phone.save(os.path.join(args.phone, f'phone-{fr:04d}.png'))
    else:
        mids = {s[1] + s[2] // 2 for s in scenes}
        extra = {int(x) for x in args.phone_frames.split(',') if x.strip()}
        for fr, g in iter_video(args.input, max(1, args.step)):
            sid = scene_of(fr, scenes)
            if sid is None:
                continue
            per_scene[sid].append(g)
        if args.phone:
            video_phone(args.input, mids | extra, args.phone)

    print()
    print(f'{"scene":<26} {"n":>4} {"mean":>6} {"p10":>6} {"p50":>6} {"p90":>6} {">0.3":>6}  verdict')
    print('-' * 86)
    rows = {}
    all_means = []
    fails = 0
    for sid, _start, _dur in scenes:
        gs = per_scene[sid]
        if not gs:
            print(f'{sid:<26} {0:>4}      -      -      -      -      -  (no frames)')
            continue
        # film-style stats: pool all pixels of the scene's sampled frames
        pooled = np.stack(gs)
        st = stats(pooled)
        all_means.extend(float(g.mean()) for g in gs)
        is_s04 = sid.startswith('s04')
        why = []
        if is_s04:
            if st['mean'] < TARGET_S04:
                why.append(f'mean<{TARGET_S04}')
        else:
            if st['mean'] < TARGET_MEAN:
                why.append(f'mean<{TARGET_MEAN}')
            if st['bright'] < TARGET_BRIGHT:
                why.append(f'>0.3<{TARGET_BRIGHT}')
        if st['p10'] > TARGET_P10:
            why.append(f'p10>{TARGET_P10}')
        verdict = 'PASS' if not why else 'FAIL ' + ', '.join(why)
        fails += bool(why)
        rows[sid] = {**st, 'n': len(gs), 'pass': not why}
        print(f'{sid:<26} {len(gs):>4} {st["mean"]:>6.3f} {st["p10"]:>6.3f} {st["p50"]:>6.3f} {st["p90"]:>6.3f} {st["bright"]:>6.3f}  {verdict}')
    print('-' * 86)
    film = float(np.mean(all_means)) if all_means else float('nan')
    film_ok = film >= TARGET_FILM
    print(f'film mean {film:.3f} (target >= {TARGET_FILM}) {"PASS" if film_ok else "FAIL"}   '
          f'scenes passing {len(rows) - fails}/{len(rows)}')
    if os.path.isdir(args.input):
        print('note: stills dir: the film mean is the mean of the listed stills (not frame-weighted per scene)')
    if args.json:
        with open(args.json, 'w') as fh:
            json.dump({'film_mean': film, 'scenes': rows, 'frames': [{'frame': f, 'scene': s, **st} for f, s, st in per_frame]}, fh, indent=1)
    return 0 if (film_ok and fails == 0) else 1


if __name__ == '__main__':
    sys.exit(main())
