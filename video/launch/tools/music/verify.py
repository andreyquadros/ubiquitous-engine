#!/usr/bin/env python3
"""
Measure the music bed against brief/storyboard.json "music" (we cannot listen, so we measure).

    python3 tools/music/verify.py [--dir public/audio/music] [--png out/review/music-overview.png]

Prints a JSON report: sample count, integrated loudness (pyloudnorm, BS.1770-4), true peak
(4x oversampled), per-section RMS / short-term loudness vs the energy map, the 225-239 silence,
the tail, onsets at every scripted hit (spectral flux, +-1 frame), stop-time gaps, spectrum
balance (sub, 3-5 kHz, DC, aliasing probe). Renders the overview PNG (waveform + spectrogram +
loudness with bar lines, section labels and event markers).
"""
import argparse
import json
import math
import os
import sys

import numpy as np
import soundfile as sf
import pyloudnorm as pyln
from scipy import signal

HERE = os.path.dirname(os.path.abspath(__file__))
PROJ = os.path.abspath(os.path.join(HERE, '..', '..'))
SR, FPS, SPF = 48000, 30, 1600


def S(f):
    return int(round(f * SPF))


def kweight(x):
    b1 = [1.53512485958697, -2.69169618940638, 1.19839281085285]
    a1 = [1.0, -1.69065929318241, 0.73248077421585]
    b2 = [1.0, -2.0, 1.0]
    a2 = [1.0, -1.99004745483398, 0.99007225036621]
    return signal.lfilter(b2, a2, signal.lfilter(b1, a1, x, axis=-1), axis=-1)


def loud_curve(xk, win_s, hop_s=0.05):
    """Loudness (LUFS, ungated) of a sliding window ending at each hop."""
    p = np.sum(xk ** 2, axis=0)
    c = np.concatenate([[0], np.cumsum(p)])
    w, h = int(win_s * SR), int(hop_s * SR)
    ends = np.arange(w, len(p) + 1, h)
    ms = (c[ends] - c[ends - w]) / w
    return ends / SR, -0.691 + 10 * np.log10(ms + 1e-20)


def rms_db(x):
    return 20 * math.log10(math.sqrt(float(np.mean(x ** 2))) + 1e-20)


def flux(mono, hop=128, nfft=1024):
    f, t, Z = signal.stft(mono, SR, nperseg=nfft, noverlap=nfft - hop, boundary=None, padded=False)
    mag = np.log1p(1000 * np.abs(Z))
    d = np.maximum(0, np.diff(mag, axis=1)).sum(axis=0)
    # time of each diff column = centre of the later frame
    tt = (np.arange(1, mag.shape[1]) * hop + nfft / 2) / SR
    return tt, d


def onset_near(tt, d, frame, tol_frames=2.0):
    t0 = frame / FPS
    m = (tt >= t0 - tol_frames / FPS) & (tt <= t0 + tol_frames / FPS)
    if not m.any():
        return None, 0.0
    i = np.argmax(np.where(m, d, -1))
    # onset = the rise start: first sample of the envelope attack (half-hop resolution)
    return (tt[i] - t0) * FPS, float(d[i] / (np.median(d[(tt > t0 - 1) & (tt < t0 + 1)]) + 1e-9))


def first_attack(mono, frame, search_before=3, search_after=3, blk=96):
    """Biggest 2 ms-envelope jump (dB) within +-3 frames: returns (offset in frames, jump dB)."""
    a, b = max(0, S(frame - search_before) - blk), S(frame + search_after)
    seg = mono[a:b]
    nb = len(seg) // blk
    e = 10 * np.log10(np.mean(seg[: nb * blk].reshape(nb, blk) ** 2, axis=1) + 1e-14)
    j = np.diff(e)
    i = int(np.argmax(j))
    return (a + (i + 1) * blk) / SPF - frame, float(j[i])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--dir', default=os.path.join(PROJ, 'public', 'audio', 'music'))
    ap.add_argument('--png', default=os.path.join(PROJ, 'out', 'review', 'music-overview.png'))
    args = ap.parse_args()
    board = json.load(open(os.path.join(PROJ, 'brief', 'storyboard.json')))['music']
    path = os.path.join(args.dir, 'music.wav')
    info = sf.info(path)
    x, sr = sf.read(path, dtype='float64', always_2d=True)
    x = x.T
    rep = {'file': path, 'samples': int(x.shape[1]), 'expectedSamples': 1560 * SPF, 'sr': sr,
           'channels': info.channels, 'subtype': info.subtype}
    meter = pyln.Meter(SR)
    rep['integratedLUFS'] = round(meter.integrated_loudness(x.T), 2)
    up = signal.resample_poly(x, 4, 1, axis=-1)
    rep['truePeakDBTP'] = round(20 * math.log10(np.max(np.abs(up))), 2)
    rep['samplePeakDBFS'] = round(20 * math.log10(np.max(np.abs(x))), 2)
    rep['dcOffset'] = [float(f'{v:.2e}') for v in x.mean(axis=1)]
    mono = x.mean(axis=0)
    xk = kweight(x)
    tS, lS = loud_curve(xk, 3.0)
    tM, lM = loud_curve(xk, 0.4)

    # sections
    secs = []
    for s in board['sections']:
        f0 = (s['startBar'] - 1) * 60
        f1 = f0 + s['bars'] * 60
        a, b = S(f0), S(min(f1, 1556 if f1 >= 1560 else f1))
        if f0 == 0:
            b = S(225)
        seg = x[:, a:b]
        m = (tM > f0 / FPS + 0.4) & (tM <= min(f1, 1500) / FPS)
        secs.append({'name': s['name'], 'frames': [f0, f1 - 1], 'energy': s['energy'],
                     'rmsDBFS': round(rms_db(seg), 1),
                     'momentaryLUFSmedian': round(float(np.median(lM[m])), 1) if m.any() else None,
                     'momentaryLUFSmax': round(float(np.max(lM[m])), 1) if m.any() else None})
    rep['sections'] = secs
    # per bar RMS
    rep['barRmsDBFS'] = [round(rms_db(x[:, S(60 * i): S(60 * i + 60)]), 1) if np.any(x[:, S(60 * i): S(60 * i + 60)]) else None
                         for i in range(26)]

    sil = x[:, S(225): S(240)]
    rep['silence225to239'] = {'rmsDBFS': rms_db(sil) if np.any(sil) else '-inf', 'maxAbs': float(np.max(np.abs(sil))),
                              'lastNonZeroBefore240Frame': None}
    nz = np.nonzero(np.any(x[:, : S(240)] != 0, axis=0))[0]
    rep['silence225to239']['lastNonZeroBefore240Frame'] = round(nz[-1] / SPF, 3) if len(nz) else None
    nz240 = np.nonzero(np.any(x[:, S(225):] != 0, axis=0))[0]
    rep['firstNonZeroAfter225Frame'] = round((S(225) + nz240[0]) / SPF, 3)
    nzall = np.nonzero(np.any(x != 0, axis=0))[0]
    rep['firstNonZeroSample'] = int(nzall[0])
    rep['lastNonZeroFrame'] = round(nzall[-1] / SPF, 3)
    rep['rmsPreStop220to224'] = round(rms_db(x[:, S(220): S(225)]), 1)
    rep['tailRms1540to1555'] = round(rms_db(x[:, S(1540): S(1556)]), 1) if np.any(x[:, S(1540): S(1556)]) else '-inf'

    # onsets
    tt, d = flux(mono)
    hits = [0, 90, 105, 150, 180, 240, 255, 300, 360, 420, 510, 525, 540, 585, 660, 690, 810, 900, 945, 960, 990,
            1035, 1050, 1320, 1380, 1395, 1440, 1455]
    ons = []
    for h in hits:
        off, strength = onset_near(tt, d, h)
        att, jump = first_attack(mono, h)
        ons.append({'frame': h, 'fluxPeakOffsetFrames': round(off, 2) if off is not None else None,
                    'fluxVsLocalMedian': round(strength, 1),
                    'envelopeJumpOffsetFrames': round(att, 3), 'envelopeJumpDb': round(jump, 1)})
    rep['onsets'] = ons
    # stop-time gaps: level between stabs vs the groove bar before
    gaps = {}
    ref = rms_db(x[:, S(420): S(480)])
    for a, b in ((518, 525), (533, 540), (668, 690), (1043, 1050)):
        gaps[f'{a}-{b}'] = round(rms_db(x[:, S(a): S(b)]) - ref, 1)
    rep['stopTimeGapsVsGrooveDb'] = gaps
    rep['thinned930to959VsGrooveDb'] = round(rms_db(x[:, S(930): S(960)]) - rms_db(x[:, S(900): S(930)]), 1)
    rep['frame675'] = {'rms670to675': round(rms_db(x[:, S(670): S(675)]), 1),
                       'rms675to680': round(rms_db(x[:, S(675): S(680)]), 1)}

    # spectrum balance on the loud part (240-1139)
    seg = mono[S(240): S(1140)]
    f, P = signal.welch(seg, SR, nperseg=16384)

    def band(lo, hi):
        m = (f >= lo) & (f < hi)
        return 10 * math.log10(np.sum(P[m]) + 1e-20)
    tot = band(20, 20000)
    rep['bandsRelTotalDb'] = {k: round(band(*v) - tot, 1) for k, v in {
        'sub 30-60': (30, 60), 'bass 60-150': (60, 150), 'lowmid 150-500': (150, 500), 'mid 500-1k': (500, 1000),
        '1-3k': (1000, 3000), '3-5k': (3000, 5000), '5-10k': (5000, 10000), '10-16k': (10000, 16000),
        '16-20k': (16000, 20000), 'below 25': (1, 25)}.items()}
    # spectral density per octave (dB/Hz) around 3-5 kHz vs neighbours: a buildup would show as a bump
    dens = lambda lo, hi: 10 * math.log10(np.mean(P[(f >= lo) & (f < hi)]) + 1e-30)
    rep['densityDbPerHz'] = {'1.5-3k': round(dens(1500, 3000), 1), '3-5k': round(dens(3000, 5000), 1),
                             '5-8k': round(dens(5000, 8000), 1)}

    # aliasing probe: polyBLEP saw at 1760 Hz, largest non-harmonic component vs fundamental
    sys.path.insert(0, HERE)
    import compose as C
    n = SR
    y = C.saw_os(1760.0, n) * np.hanning(n)
    Y = 20 * np.log10(np.abs(np.fft.rfft(y)) + 1e-12)
    Y -= Y.max()
    fr = np.fft.rfftfreq(n, 1 / SR)
    harm = np.zeros_like(fr, dtype=bool)
    for k in range(1, int(24000 / 1760) + 1):
        harm |= np.abs(fr - 1760 * k) < 12
    rep['aliasProbeSaw1760_2xPolyBLEP'] = {'worstNonHarmonicDb': round(float(Y[~harm & (fr > 20)].max()), 1),
                                'worstNonHarmonicBelow5kDb': round(float(Y[~harm & (fr > 20) & (fr < 5000)].max()), 1)}

    # stems
    st = {}
    for name in ('drums', 'bass', 'harmony', 'lead', 'fx'):
        p = os.path.join(args.dir, 'stems', f'{name}.wav')
        if os.path.exists(p):
            i = sf.info(p)
            s_, _ = sf.read(p, dtype='float64', always_2d=True)
            st[name] = {'frames': i.frames, 'peakDBFS': round(20 * math.log10(np.max(np.abs(s_)) + 1e-20), 1),
                        'rmsDBFS': round(rms_db(s_), 1),
                        'silent225to239': bool(not np.any(s_[S(225): S(240)]))}
    rep['stems'] = st
    alt = os.path.join(args.dir, 'music-drop-nocrash.wav')
    if os.path.exists(alt):
        rep['altNoCrashFrames'] = sf.info(alt).frames

    print(json.dumps(rep, indent=1, ensure_ascii=False))
    if args.png:
        plot(x, mono, tS, lS, tM, lM, board, args.png, os.path.join(args.dir, 'markers.json'))


def plot(x, mono, tS, lS, tM, lM, board, png, markers_path):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    os.makedirs(os.path.dirname(png), exist_ok=True)
    fig, axs = plt.subplots(3, 1, figsize=(26, 13), sharex=True, gridspec_kw={'height_ratios': [1.1, 2.2, 1.0]})
    t = np.arange(x.shape[1]) / SR
    hop = 240
    nb = x.shape[1] // hop
    env = np.abs(x[:, : nb * hop]).reshape(2, nb, hop).max(axis=2)
    tt = np.arange(nb) * hop / SR
    axs[0].fill_between(tt, env[0], -env[1], color='#3b6fd8', lw=0)
    axs[0].set_ylim(-1.05, 1.05)
    axs[0].set_ylabel('L / R peak')
    f, ts, Z = signal.stft(mono, SR, nperseg=4096, noverlap=4096 - 480)
    Zdb = 20 * np.log10(np.abs(Z) + 1e-9)
    axs[1].pcolormesh(ts, f, Zdb, vmin=Zdb.max() - 90, vmax=Zdb.max(), cmap='magma', shading='auto')
    axs[1].set_yscale('symlog', linthresh=200)
    axs[1].set_ylim(25, 20000)
    axs[1].set_yticks([30, 60, 100, 200, 500, 1000, 2000, 5000, 10000, 20000])
    axs[1].set_yticklabels(['30', '60', '100', '200', '500', '1k', '2k', '5k', '10k', '20k'])
    axs[1].set_ylabel('Hz')
    axs[2].plot(tM, lM, color='#999', lw=0.8, label='momentary (400 ms)')
    axs[2].plot(tS, lS, color='#d83b3b', lw=1.6, label='short-term (3 s)')
    axs[2].axhline(-14, color='k', ls=':', lw=0.8)
    axs[2].set_ylim(-40, -4)
    axs[2].set_ylabel('LUFS')
    axs[2].legend(loc='lower left', fontsize=8)
    for ax in axs:
        for b in range(27):
            ax.axvline(b * 2.0, color='#00000033' if ax is not axs[1] else '#ffffff55', lw=0.7)
        ax.axvspan(225 / FPS, 240 / FPS, color='#2ca02c33')
    for s in board['sections']:
        f0 = (s['startBar'] - 1) * 60
        axs[0].text(f0 / FPS + 0.05, 0.93, f"{s['name']}\nbars {s['startBar']}-{s['startBar'] + s['bars'] - 1} · E{s['energy']}",
                    fontsize=8, va='top')
        for ax in axs:
            ax.axvline(f0 / FPS, color='#d8a03b', lw=1.8)
    if os.path.exists(markers_path):
        mk = json.load(open(markers_path))
        for e in mk['events']:
            axs[2].axvline(e['frame'] / FPS, color='#3b6fd8', lw=0.6, alpha=0.6)
            axs[2].text(e['frame'] / FPS, -39, str(int(e['frame'])), fontsize=6, rotation=90, va='bottom', color='#3b6fd8')
    for b in range(26):
        axs[2].text(b * 2 + 0.05, -6, str(b + 1), fontsize=7, color='#555')
    axs[2].set_xlabel('seconds (bar lines every 2 s = 60 frames; green = 225-239 silence)')
    axs[2].set_xlim(0, 52)
    fig.suptitle('ubiqX launch — music.wav overview (A minor, 120 BPM, 26 bars, 1560 frames)')
    fig.tight_layout()
    fig.savefig(png, dpi=80)


if __name__ == '__main__':
    main()
