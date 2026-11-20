#!/usr/bin/env python3
"""
Master the launch film's audio and mux it onto the rendered picture.

  python3 tools/master/master.py --video out/launch-v1.mp4 --mix out/audio/mix.wav --out out/launch-v1-master.mp4

--mix is the PCM mix from an audio-only render of the SAME code
(`REMOTION_AUDIO_ONLY=1 npx remotion render src/index.ts Launch out/audio/mix.wav --codec=wav`),
which is sample-exact (the AAC inside Remotion's MP4 decodes ~2048 samples late).

Chain: 4x-oversampled look-ahead peak limiter (latency-compensated, catches the
isolated impact transients only) -> two-pass loudnorm I=-14 TP=-1, linear
(LRA untouched) -> AAC-LC 48 kHz 320 kbps; video stream copied; +faststart.
"""
import argparse, json, re, subprocess, sys, os

FF = '/usr/local/bin/ffmpeg'
ap = argparse.ArgumentParser()
ap.add_argument('--video', required=True)
ap.add_argument('--mix', required=True)
ap.add_argument('--out', required=True)
ap.add_argument('--limit-db', type=float, default=-7.5, help='pre-limiter ceiling (dBFS, sample peak at 4x)')
ap.add_argument('--I', type=float, default=-14.0)
ap.add_argument('--TP', type=float, default=-1.0)
a = ap.parse_args()

tmp = os.path.splitext(a.out)[0]
lim = tmp + '.lim.wav'
norm = tmp + '.norm.wav'
limit = 10 ** (a.limit_db / 20)
pre = f'aresample=192000,alimiter=limit={limit:.4f}:attack=1:release=60:level=false:latency=true,aresample=48000,atrim=end_sample=2496000,asetpts=N/SR/TB'
subprocess.run([FF, '-hide_banner', '-y', '-i', a.mix, '-af', pre, '-c:a', 'pcm_f32le', lim], check=True, capture_output=True)

def loudnorm(extra, src, dst=None):
    cmd = [FF, '-hide_banner', '-nostats', '-y', '-i', src, '-af', f'loudnorm=I={a.I}:TP={a.TP}:LRA=50:print_format=json{extra}']
    cmd += ['-ar', '48000', '-c:a', 'pcm_f32le', dst] if dst else ['-f', 'null', '-']
    r = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return json.loads(re.findall(r'\{[^{}]*\}', r.stderr)[-1])

m = loudnorm('', lim)
print('pass 1:', {k: m[k] for k in ('input_i', 'input_tp', 'input_lra', 'input_thresh')})
extra = (f":measured_I={m['input_i']}:measured_TP={m['input_tp']}:measured_LRA={m['input_lra']}"
         f":measured_thresh={m['input_thresh']}:offset={m['target_offset']}:linear=true")
m2 = loudnorm(extra, lim, norm)
print('pass 2:', {k: m2[k] for k in ('output_i', 'output_tp', 'output_lra', 'normalization_type')})
if m2['normalization_type'] != 'linear':
    print('WARNING: loudnorm fell back to dynamic mode', file=sys.stderr)

subprocess.run([FF, '-hide_banner', '-y', '-i', a.video, '-i', norm, '-map', '0:v:0', '-map', '1:a:0',
                '-c:v', 'copy', '-c:a', 'aac', '-b:a', '320k', '-ar', '48000', '-ac', '2',
                '-movflags', '+faststart', a.out], check=True, capture_output=True)
for f in (lim, norm):
    os.remove(f)
print('wrote', a.out)
