#!/usr/bin/env python3
"""
Crash-tolerant, resumable picture render of the Launch film.

  python3 tools/master/render.py --out out/launch-v2.mp4 [--chunk 260] [--concurrency 2]

Why: a single `npx remotion render` of all 1560 frames is all-or-nothing. On
this shared 4-core box a browser crash (e.g. another job's stills browser
starving memory/CPU; v2 died at frame 51 with concurrency 3) throws away the
whole run. This renders the film in --chunk frame ranges (muted: the master
takes its audio from the PCM audio-only render anyway), keeps finished chunks
on disk (<out>.parts/), retries a failed chunk at concurrency 1, and joins the
chunks with the ffmpeg concat demuxer (stream copy, no re-encode: every chunk
is an independent H.264 stream with the same remotion.config.ts settings and
starts on an IDR frame). Re-running the same command resumes.

Then:
  REMOTION_AUDIO_ONLY=1 npx remotion render src/index.ts Launch out/audio/mix-v2.wav --codec=wav
  python3 tools/master/master.py --video out/launch-v2.mp4 --mix out/audio/mix-v2.wav --out out/launch-v2-master.mp4
"""
import argparse, os, re, subprocess, sys, time

FF = '/usr/local/bin/ffmpeg'
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ap = argparse.ArgumentParser()
ap.add_argument('--out', required=True)
ap.add_argument('--comp', default='Launch')
ap.add_argument('--total', type=int, default=1560, help='film length in frames (src/timeline.ts TOTAL)')
ap.add_argument('--chunk', type=int, default=260)
ap.add_argument('--concurrency', type=int, default=2)
ap.add_argument('--retries', type=int, default=2, help='extra attempts per chunk (at concurrency 1)')
a = ap.parse_args()

out = os.path.join(ROOT, a.out) if not os.path.isabs(a.out) else a.out
parts = out + '.parts'
os.makedirs(parts, exist_ok=True)


def frames_in(path):
    """Video packet count of a file (one framecrc line per packet; no ffprobe on this box)."""
    r = subprocess.run([FF, '-v', 'error', '-i', path, '-map', '0:v:0', '-c', 'copy', '-f', 'framecrc', '-'],
                       capture_output=True, text=True)
    if r.returncode != 0:
        return -1
    return sum(1 for ln in r.stdout.splitlines() if ln and not ln.startswith('#'))


chunks = [(s, min(s + a.chunk, a.total) - 1) for s in range(0, a.total, a.chunk)]
t0 = time.time()
for s, e in chunks:
    dst = os.path.join(parts, f'{s:04d}-{e:04d}.mp4')
    want = e - s + 1
    if os.path.exists(dst) and frames_in(dst) == want:
        print(f'[{s}-{e}] done already', flush=True)
        continue
    for attempt in range(a.retries + 1):
        conc = a.concurrency if attempt == 0 else 1
        tmp = dst + '.tmp.mp4'
        log = dst + '.log'
        ts = time.time()
        with open(log, 'w') as lf:
            rc = subprocess.run(['npx', 'remotion', 'render', 'src/index.ts', a.comp, tmp, f'--frames={s}-{e}',
                                 f'--concurrency={conc}', '--muted'], cwd=ROOT, stdout=lf, stderr=subprocess.STDOUT).returncode
        n = frames_in(tmp) if os.path.exists(tmp) else -1
        if rc == 0 and n == want:
            os.replace(tmp, dst)
            print(f'[{s}-{e}] ok  {want} frames, conc {conc}, {time.time() - ts:.0f}s', flush=True)
            break
        print(f'[{s}-{e}] FAILED attempt {attempt + 1} (rc {rc}, {n}/{want} frames, conc {conc}); see {log}', flush=True)
        time.sleep(20)
    else:
        sys.exit(f'chunk {s}-{e} failed {a.retries + 1} times; re-run to resume')

lst = os.path.join(parts, 'concat.txt')
with open(lst, 'w') as f:
    for s, e in chunks:
        f.write(f"file '{s:04d}-{e:04d}.mp4'\n")
subprocess.run([FF, '-hide_banner', '-y', '-f', 'concat', '-safe', '0', '-i', lst, '-map', '0:v:0', '-c', 'copy',
                '-movflags', '+faststart', out], check=True, capture_output=True)
n = frames_in(out)
print(f'wrote {out}: {n} frames ({"OK" if n == a.total else "MISMATCH, expected " + str(a.total)}), {time.time() - t0:.0f}s total')
sys.exit(0 if n == a.total else 1)
