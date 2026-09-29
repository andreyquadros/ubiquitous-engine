#!/usr/bin/env python3
"""
QA of a rendered master: contact sheets (one frame every 10, 4x3 tiles) and an
audio overview (waveform + spectrogram + short-term loudness, scene boundaries).

  python3 tools/master/qa.py out/launch-v1-master.mp4 out/review/v1
"""
import json, os, subprocess, sys
import numpy as np
FF = '/usr/local/bin/ffmpeg'
src, outdir = sys.argv[1], sys.argv[2]
os.makedirs(outdir, exist_ok=True)
root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sb = json.load(open(os.path.join(root, 'brief/storyboard.json')))
scenes = [(s['id'], s['startFrame'], s['durationInFrames']) for s in sb['scenes']]
font = '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'

if '--no-sheets' not in sys.argv:
    for f in os.listdir(outdir):
        if f.startswith('contact-') and f.endswith('.jpg'):
            os.remove(os.path.join(outdir, f))
    vf = (f"drawtext=fontfile={font}:text='%{{frame_num}}':x=8:y=8:fontsize=28:fontcolor=yellow:box=1:boxcolor=black@0.7,"
          "select='not(mod(n\\,10))',scale=640:360,tile=4x3:padding=4:color=magenta")
    subprocess.run([FF, '-hide_banner', '-y', '-i', src, '-vf', vf, '-fps_mode', 'vfr', '-q:v', '3',
                    os.path.join(outdir, 'contact-%02d.jpg')], check=True, capture_output=True)

# audio
wav = os.path.join(outdir, '_a.wav')
subprocess.run([FF, '-hide_banner', '-y', '-i', src, '-vn', '-ac', '2', '-ar', '48000', '-c:a', 'pcm_f32le', wav], check=True, capture_output=True)
import soundfile as sf
import matplotlib; matplotlib.use('Agg')
import matplotlib.pyplot as plt
a, sr = sf.read(wav); os.remove(wav)
mono = a.mean(1); n = len(mono); spf = sr // 30
frames = n // spf
fr = np.arange(frames)
blk = mono[:frames * spf].reshape(frames, spf)
pk = 20 * np.log10(np.abs(blk).max(1) + 1e-9)
rms = 20 * np.log10(np.sqrt((blk ** 2).mean(1)) + 1e-9)
fig, ax = plt.subplots(3, 1, figsize=(26, 12), sharex=True, gridspec_kw={'height_ratios': [1.2, 2, 1]})
t = np.arange(n) / sr * 30
step = 40
ax[0].fill_between(t[::step], a[::step, 0], -a[::step, 1] * 0 + a[::step, 0] * 0, color='#4d8dff', lw=0)
ax[0].plot(t[::step], a[::step, 0], color='#4d8dff', lw=0.3); ax[0].plot(t[::step], a[::step, 1], color='#ff7a1f', lw=0.3, alpha=0.6)
ax[0].set_ylim(-1, 1); ax[0].set_ylabel('waveform L/R')
ax[1].specgram(mono, NFFT=2048, Fs=30 * 2048 / 2048 * sr / 30, noverlap=1536, cmap='magma', vmin=-130, vmax=-20, scale='dB')
# specgram's x axis is in seconds*? -> rescale: we passed Fs=sr so x is seconds; convert ticks below
ax[1].cla()
Pxx, freqs, bins, im = ax[1].specgram(mono, NFFT=2048, Fs=sr, noverlap=1536, cmap='magma', vmin=-130, vmax=-25)
im.set_extent([0, n / sr * 30, freqs[0], freqs[-1]])
ax[1].set_yscale('symlog', linthresh=200); ax[1].set_ylim(30, 20000); ax[1].set_ylabel('Hz')
ax[2].plot(fr, pk, color='#e8edf9', lw=0.6, label='peak dBFS / frame')
ax[2].plot(fr, rms, color='#2ecc8f', lw=0.8, label='RMS dBFS / frame')
# short-term loudness (3 s) via ffmpeg ebur128 would be nicer; RMS 1 s moving average as a proxy
k = 30; ma = 10 * np.log10(np.convolve(10 ** (rms / 10), np.ones(k) / k, 'same') + 1e-12)
ax[2].plot(fr, ma, color='#ff7a1f', lw=1.2, label='RMS 1 s avg')
ax[2].set_ylim(-70, 0); ax[2].legend(loc='lower left', fontsize=9, facecolor='#222', labelcolor='w'); ax[2].set_xlabel('frame (30 fps)')
for axx in ax:
    axx.set_facecolor('#0a0d16')
    for sid, st, d in scenes:
        axx.axvline(st, color='#ffffff', lw=0.6, alpha=0.5)
    axx.axvspan(225, 240, color='#ff4d6d', alpha=0.18)
for sid, st, d in scenes:
    ax[0].text(st + 2, 0.88, sid[:3], color='w', fontsize=9, va='top')
    ax[0].text(st + 2, -0.95, str(st), color='#aaa', fontsize=7)
ax[0].set_title(f'{os.path.basename(src)} - audio overview (scene starts white; 225-239 silence band red)', color='w')
fig.patch.set_facecolor('#11151f')
for axx in ax:
    axx.tick_params(colors='#ccc'); axx.yaxis.label.set_color('#ccc'); axx.xaxis.label.set_color('#ccc')
ax[2].set_xlim(0, 1560); ax[2].set_xticks(range(0, 1561, 60))
plt.tight_layout(); plt.savefig(os.path.join(outdir, 'audio.png'), dpi=80)
sil = blk[225:240]
print('audio frames', frames, ' silence 225-239 peak dBFS', round(float(20 * np.log10(np.abs(sil).max() + 1e-12)), 1),
      ' tail 1556-1559 peak', round(float(20 * np.log10(np.abs(blk[1556:1560]).max() + 1e-12)), 1))
