#!/usr/bin/env python3
"""
ubiqX AI launch video - synthesized SFX library (no samples, no downloads).

    python3 video/launch/tools/sfx/synth_sfx.py                # build all + validate + manifest + contact sheet
    python3 video/launch/tools/sfx/synth_sfx.py --only whoosh  # rebuild sounds whose file name matches a regex
    python3 video/launch/tools/sfx/synth_sfx.py --no-sheet     # skip the contact sheet
    python3 video/launch/tools/sfx/synth_sfx.py --check        # only re-validate files on disk + rewrite manifest

Needs numpy + scipy (+ matplotlib for the contact sheet):  pip install scipy matplotlib

Output (video/launch/public/audio/sfx/):
    <name>_<variation>.wav   48 kHz / stereo / 16-bit PCM, peak -1 dBFS, DC removed, faded in/out
    <style-guide alias>.wav  byte copies under the names in brief/style.md 7.5 (impact.wav, pop+1.wav, ...)
    sfx-manifest.json        one row per file: timing (hit_offset_s), levels, suggested use
    _contact.png             waveform + log-frequency spectrogram of every primary sound

Timing conventions (120 BPM, 30 fps):
    hit_offset_s  - time of the main transient / loudest point inside the file. To land it on visual event
                    frame E, start the cue at  at = E - round(hit_offset_s * 30)  (style: 0..+1 frame late is fine).
    end_is_downbeat - risers / reverse swells: the file ends exactly on its last sample at full intensity, so
                    start it at  at = dropFrame - duration_s * 30  (riser_1bar: -60 f, riser_2bar: -120 f).

Determinism: every random source is seeded from crc32("<layer>:<name>_<variation>"); re-running the script
reproduces the library (bit-identical on the same numpy/scipy versions).
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import zlib
from pathlib import Path

import numpy as np
from scipy import signal
from scipy.io import wavfile

SR = 48000
BPM = 120
BEAT = 60.0 / BPM  # 0.5 s  (15 frames)
BAR = 4 * BEAT  # 2.0 s  (60 frames)
FPS = 30
PEAK_DBFS = -1.0
TP_CEIL = -0.5  # dBTP ceiling (inter-sample peaks)

HERE = Path(__file__).resolve()
LAUNCH = HERE.parents[2]  # video/launch
OUT = LAUNCH / 'public' / 'audio' / 'sfx'

# ============================================================================ basic helpers


def ns(d: float) -> int:
    return int(round(d * SR))


def tax(n: int) -> np.ndarray:
    return np.arange(n) / SR


def db2a(d):
    return 10.0 ** (np.asarray(d, float) / 20.0)


def a2db(a: float) -> float:
    return float(20.0 * np.log10(max(float(a), 1e-12)))


def midi(m: float) -> float:
    return 440.0 * 2.0 ** ((m - 69) / 12.0)


def rng_for(*parts) -> np.random.Generator:
    return np.random.default_rng(zlib.crc32(':'.join(map(str, parts)).encode()))


def rms(x) -> float:
    return float(np.sqrt(np.mean(np.square(x)) + 1e-24))


def unit(x):
    """Normalise to unit RMS (keeps layer gains meaningful)."""
    return x / rms(x)


def st(x):
    """Mono (n,) -> stereo (2, n)."""
    x = np.asarray(x, float)
    return np.stack([x, x]) if x.ndim == 1 else x


def smooth(x, ms: float):
    """Hann-window smoothing (zero-phase, edge padded)."""
    w = max(3, int(ms * SR / 1000.0)) | 1
    h = np.hanning(w + 2)[1:-1]
    h /= h.sum()
    pad = w // 2
    return np.convolve(np.pad(x, pad, mode='edge'), h, mode='valid')


def colored(rng, n, slope=0.0, floor_hz=20.0):
    """Gaussian noise with a spectral slope in dB/octave (0 white, -3 pink, -6 brown); unit RMS."""
    x = rng.standard_normal(n)
    if slope == 0:
        return unit(x)
    X = np.fft.rfft(x)
    f = np.maximum(np.fft.rfftfreq(n, 1.0 / SR), floor_hz)
    X *= (f / 1000.0) ** (slope / 6.0206)
    return unit(np.fft.irfft(X, n))


def stereo_noise(rng, n, corr=0.5, slope=0.0):
    """Partially correlated stereo noise: corr=1 mono, corr=0 fully decorrelated (wide)."""
    c, l, r = colored(rng, n, slope), colored(rng, n, slope), colored(rng, n, slope)
    a, b = np.sqrt(corr), np.sqrt(1.0 - corr)
    return np.stack([a * c + b * l, a * c + b * r])


def curve(t, pts, log=False, ease=True):
    """Breakpoint curve through [(time, value), ...]; smoothstep per segment; log=True interpolates log(value)."""
    tp = np.array([p[0] for p in pts], float)
    vp = np.array([p[1] for p in pts], float)
    v = np.log(vp) if log else vp
    idx = np.clip(np.searchsorted(tp, t, side='right') - 1, 0, len(tp) - 2)
    t0, t1 = tp[idx], tp[idx + 1]
    u = np.clip((t - t0) / np.maximum(t1 - t0, 1e-9), 0.0, 1.0)
    if ease:
        u = u * u * (3 - 2 * u)
    y = v[idx] + (v[idx + 1] - v[idx]) * u
    return np.exp(y) if log else y


def pan_move(t, a, b, tc, w):
    """Pass-by pan trajectory from a to b, fastest at tc (tanh)."""
    return a + (b - a) * (0.5 + 0.5 * np.tanh((t - tc) / w))


def pan(x, p):
    """Equal-power pan (mono in) or balance (stereo in); p in [-1, 1], centre = unity on both sides."""
    th = (np.clip(p, -1, 1) + 1) * np.pi / 4
    gl, gr = np.cos(th) * np.sqrt(2), np.sin(th) * np.sqrt(2)
    x = np.asarray(x, float)
    if x.ndim == 1:
        return np.stack([x * gl, x * gr])
    return np.stack([x[0] * gl, x[1] * gr])


def vdelay(x, d_samples):
    """Fractional, time-varying delay (linear interpolation)."""
    i = np.arange(len(x), dtype=float)
    return np.interp(i - d_samples, i, x, left=0.0)


# ---------------------------------------------------------------------------- envelopes


def bell(t, tp, w_pre, w_post, p_pre=2.0, p_post=2.0):
    """Pass-by envelope: 1 at tp and (1 + (dt/w)^2)^(-p/2) either side. Smooth (no cusp) at the peak."""
    dt = t - tp
    w = np.where(dt < 0, w_pre, w_post)
    p = np.where(dt < 0, p_pre, p_post)
    return (1.0 + (dt / w) ** 2) ** (-p / 2.0)


def attack_decay(t, t0=0.0, atk=0.002, tau=0.1):
    """Raised-cosine attack from exactly 0 at t0, then exponential decay."""
    tt = t - t0
    ttc = np.clip(tt, 0.0, None)
    a = np.where(ttc < atk, 0.5 - 0.5 * np.cos(np.pi * ttc / atk), 1.0)
    return np.where(tt < 0, 0.0, a * np.exp(-ttc / tau))


# ---------------------------------------------------------------------------- filters


def svf(x, fc, q):
    """Zavalishin TPT state-variable filter with per-sample cutoff / Q (stable under fast modulation).
    Returns (lowpass, bandpass normalised to unity peak gain, highpass)."""
    n = len(x)
    fc = np.broadcast_to(np.asarray(fc, float), (n,))
    q = np.broadcast_to(np.asarray(q, float), (n,))
    g = np.tan(np.pi * np.clip(fc, 10.0, 0.45 * SR) / SR)
    k = 1.0 / q
    a1 = 1.0 / (1.0 + g * (g + k))
    a2 = g * a1
    a3 = g * a2
    lp = [0.0] * n
    bp = [0.0] * n
    ic1 = ic2 = 0.0
    i = 0
    for v0, A1, A2, A3 in zip(np.asarray(x, float).tolist(), a1.tolist(), a2.tolist(), a3.tolist()):
        v3 = v0 - ic2
        v1 = A1 * ic1 + A2 * v3
        v2 = ic2 + A2 * ic1 + A3 * v3
        ic1 = 2.0 * v1 - ic1
        ic2 = 2.0 * v2 - ic2
        lp[i] = v2
        bp[i] = v1
        i += 1
    lp = np.array(lp)
    bp = np.array(bp)
    hp = x - k * bp - lp
    return lp, bp * k, hp


def butter(x, kind, fc, order=2):
    sos = signal.butter(order, fc, btype=kind, fs=SR, output='sos')
    return signal.sosfilt(sos, x, axis=-1)


def rbj(kind, f0, gain_db=0.0, q=0.707):
    """RBJ cookbook biquad as a single SOS row."""
    A = 10 ** (gain_db / 40.0)
    w = 2 * np.pi * f0 / SR
    cw, sw = np.cos(w), np.sin(w)
    al = sw / (2 * q)
    if kind == 'peak':
        b = [1 + al * A, -2 * cw, 1 - al * A]
        a = [1 + al / A, -2 * cw, 1 - al / A]
    elif kind == 'lowshelf':
        s = 2 * np.sqrt(A) * al
        b = [A * ((A + 1) - (A - 1) * cw + s), 2 * A * ((A - 1) - (A + 1) * cw), A * ((A + 1) - (A - 1) * cw - s)]
        a = [(A + 1) + (A - 1) * cw + s, -2 * ((A - 1) + (A + 1) * cw), (A + 1) + (A - 1) * cw - s]
    elif kind == 'highshelf':
        s = 2 * np.sqrt(A) * al
        b = [A * ((A + 1) + (A - 1) * cw + s), -2 * A * ((A - 1) + (A + 1) * cw), A * ((A + 1) + (A - 1) * cw - s)]
        a = [(A + 1) - (A - 1) * cw + s, 2 * ((A - 1) - (A + 1) * cw), (A + 1) - (A - 1) * cw - s]
    else:
        raise ValueError(kind)
    b = np.array(b) / a[0]
    a = np.array(a) / a[0]
    return np.concatenate([b, a])[None, :]


def eq(x, *bands):
    for kind, f0, g, q in bands:
        x = signal.sosfilt(rbj(kind, f0, g, q), x, axis=-1)
    return x


def sat(x, drive=1.5):
    """Gentle tanh saturation, level-independent (drive is relative to the signal's own peak)."""
    pk = float(np.max(np.abs(x))) + 1e-12
    return np.tanh(drive * x / pk) / np.tanh(drive) * pk


# ---------------------------------------------------------------------------- oscillators


def phase_of(f, t=None, t0=0.0):
    """Phase accumulator for a (time-varying) frequency, starting at 0 at t0."""
    f = np.asarray(f, float)
    if t is not None:
        f = np.broadcast_to(f, t.shape) * (t >= t0)
    return 2 * np.pi * np.cumsum(f) / SR


def bl_saw(ph, f, fmax=14000.0, kmax=80):
    """Band-limited sawtooth by additive synthesis with a soft harmonic roll-off near fmax (no aliasing)."""
    y = np.zeros_like(ph)
    for k in range(1, kmax + 1):
        m = np.clip((fmax - k * np.asarray(f)) / (0.2 * fmax), 0.0, 1.0)
        if not np.any(m > 0):
            break
        y += m * np.sin(k * ph) / k
    return y


def bl_square(ph, f, fmax=12000.0, kmax=31):
    y = np.zeros_like(ph)
    for k in range(1, kmax + 1, 2):
        m = np.clip((fmax - k * np.asarray(f)) / (0.2 * fmax), 0.0, 1.0)
        if not np.any(m > 0):
            break
        y += m * np.sin(k * ph) / k
    return y


# ---------------------------------------------------------------------------- space


_IR_CACHE: dict = {}


def make_ir(key, rt60=0.8, hf=0.45, bright=9000.0, predelay=0.010, length=None):
    """Synthetic stereo room: 3-band exponentially decaying noise (highs die faster), diffuse build-up,
    decorrelated L/R, unit energy."""
    ck = (key, rt60, hf, bright, predelay, length)
    if ck in _IR_CACHE:
        return _IR_CACHE[ck]
    rng = rng_for('ir', key)
    length = length or min(rt60 * 1.1, 3.0)
    n = ns(length)
    t = tax(n)
    chans = []
    for _ in range(2):
        z = rng.standard_normal(n)
        lo = butter(z, 'lowpass', 700)
        hi = butter(z, 'highpass', 3500)
        mid = z - lo - hi
        dec = lambda rt: np.exp(-6.9078 * t / rt)  # noqa: E731  (-60 dB at rt)
        ir = 1.1 * lo * dec(rt60 * 1.05) + mid * dec(rt60) + hi * dec(max(rt60 * hf, 0.05))
        ir = butter(ir, 'lowpass', bright, order=1)
        ir *= 1.0 - np.exp(-t / 0.006)
        pd = ns(predelay)
        ir = np.concatenate([np.zeros(pd), ir])[:n]
        chans.append(ir)
    ir = np.stack(chans)
    ir /= np.sqrt(np.sum(ir ** 2) / 2)
    _IR_CACHE[ck] = ir
    return ir


def reverb(x, ir, wet_db=-12.0, hp=180.0):
    """Mono-in / stereo-out convolution. The wet level is set as ENERGY relative to the dry signal."""
    x = st(x)
    n = x.shape[1]
    m = butter(0.5 * (x[0] + x[1]), 'highpass', hp)  # keep lows dry and tight
    wet = np.stack([signal.fftconvolve(m, ir[c])[:n] for c in range(2)])
    g = float(db2a(wet_db)) * rms(x) / max(rms(wet), 1e-12)
    return x + g * wet


PREROLL = 0.002  # crisp transients start 2 ms in, so the file's fade-in runs over silence, not over the click


def preroll(y, d=PREROLL):
    """Delay content by d seconds (length preserved) - keeps a designed sub-ms transient intact under the fade-in."""
    k = ns(d)
    y = np.asarray(y, float)
    return np.concatenate([np.zeros(y.shape[:-1] + (k,)), y[..., :y.shape[-1] - k]], axis=-1)


def haas(x, ms, right=True):
    d = ns(ms / 1000.0)
    x = st(x).copy()
    c = 1 if right else 0
    x[c] = np.concatenate([np.zeros(d), x[c][:-d]]) if d else x[c]
    return x


# ---------------------------------------------------------------------------- finalisation / io


def finalize(x, fade_in=0.003, fade_out=0.010, hp=16.0):
    """HPF 16 Hz + DC removal, raised-cosine fades that start/end at exactly 0, peak -> -1 dBFS."""
    x = st(np.asarray(x, float)).copy()
    if not np.all(np.isfinite(x)):
        raise ValueError('non-finite samples before finalisation')
    x = butter(x, 'highpass', hp)
    x -= x.mean(axis=1, keepdims=True)
    fi, fo = ns(fade_in), ns(fade_out)
    if fi:
        x[:, :fi] *= 0.5 - 0.5 * np.cos(np.pi * np.arange(fi) / fi)
    if fo:
        x[:, -fo:] *= 0.5 + 0.5 * np.cos(np.pi * (np.arange(fo) + 1) / fo)
    # residual DC of short transients (a decaying sine starting at phase 0 has a net offset): subtract a
    # mean-compensating Hann bump - zero at both ends, so the file still starts/ends on exactly 0.
    w = np.hanning(x.shape[1])
    x -= (x.sum(axis=1, keepdims=True) / w.sum()) * w[None, :]
    # sample peak -> -1 dBFS, but never let the 4x-oversampled (true) peak exceed -0.5 dBTP
    tp = float(np.abs(signal.resample_poly(x, 4, 1, axis=1)).max())
    x *= min(float(db2a(PEAK_DBFS)) / float(np.max(np.abs(x))), float(db2a(TP_CEIL)) / tp)
    if not np.all(np.isfinite(x)):
        raise ValueError('non-finite samples after finalisation')
    return x


def write_wav(path: Path, x):
    pcm = np.clip(np.round(x.T * 32767.0), -32768, 32767).astype(np.int16)
    wavfile.write(str(path), SR, pcm)


# ============================================================================ sound engines


def whoosh(key, dur, tp, w_pre, w_post, p_pre, p_post, fc, q, pan_=(0, 0, 0, 1), slope=-1.5, corr=0.45,
           air=0.4, air_mult=1.7, body=0.35, body_mult=0.22, whistle=0.0, whistle_q=9.0,
           flange=0.0, flange_ms=(7.0, 0.7), rumble=0.0, rev=(0.6, -15.0)):
    """Air movement: resonant band-pass sweep over coloured noise + air (HP) + body (LP) [+ whistle, jet
    flanger, sub rumble], pass-by envelope, moving pan, synthetic room."""
    rng = rng_for('whoosh', key)
    n = ns(dur)
    t = tax(n)
    env = bell(t, tp, w_pre, w_post, p_pre, p_post)
    f = curve(t, fc, log=True)
    qq = curve(t, q)
    src = stereo_noise(rng, n, corr, slope)
    air_src = stereo_noise(rng, n, 0.15, 0.0)
    body_src = stereo_noise(rng, n, 0.6, -4.5)
    rum_src = colored(rng, n, -6.0)
    if flange:
        d = curve(t, [(0, flange_ms[0]), (tp, flange_ms[1]), (dur, flange_ms[0] * 0.8)]) * SR / 1000.0
    out = np.zeros((2, n))
    for c in range(2):
        _, bp, _ = svf(src[c], f, qq)
        layer = unit(bp)
        if whistle:
            _, wb, _ = svf(src[c], f * 1.02, whistle_q)
            layer += whistle * unit(wb)
        if air:
            _, _, ah = svf(air_src[c], np.clip(f * air_mult, 1800.0, 15000.0), 0.7)
            layer += air * unit(ah) * env ** 0.5
        if body:
            lp, _, _ = svf(body_src[c], np.clip(f * body_mult, 70.0, 1400.0), 0.8)
            layer += body * unit(lp)
        if flange:
            layer = layer + flange * vdelay(layer, d)
        out[c] = layer * env
    if rumble:
        lp, _, _ = svf(rum_src, 110.0, 0.8)
        out += rumble * unit(lp) * np.where(t < tp, env ** 0.7, env) * rms(out)
    out = pan(out, pan_move(t, *pan_))
    return reverb(out, make_ir(key, rev[0], hf=0.4, bright=10000), rev[1])


def impact(key, dur, f_sub=50.0, f_start=140.0, pitch_tau=0.028, sub_tau=0.25, drive=1.8,
           knock=((170, 0.06, 0.45),), trans=0.28, trans_tau=0.007, trans_band=(1500, 9000),
           mid=0.25, mid_tau=0.05, mid_band=(300, 2500), tail=(1.0, -12.0, 0.35), width=0.8):
    """Restrained cinematic hit: pitched sub thump (kick-style glide into f_sub) + damped 'knock' modes +
    short broadband transient + mid 'air punch'; room tail on everything except the sub."""
    rng = rng_for('impact', key)
    n = ns(dur)
    t = tax(n)
    f = f_sub + (f_start - f_sub) * np.exp(-t / pitch_tau)
    sub = np.sin(phase_of(f)) * attack_decay(t, 0, 0.0015, sub_tau)
    sub = sat(sub, drive)
    kn = np.zeros(n)
    for fk, tk, ak in knock:
        fk_t = fk * (1 + 0.3 * np.exp(-t / 0.008))
        kn += ak * np.sin(phase_of(fk_t)) * attack_decay(t, 0, 0.001, tk)
    tr = unit(butter(stereo_noise(rng, n, 1 - width, 0.0), 'bandpass', trans_band))
    tr *= attack_decay(t, 0, 0.0004, trans_tau) * trans
    md = unit(butter(stereo_noise(rng, n, 0.5, -3.0), 'bandpass', mid_band))
    md *= attack_decay(t, 0, 0.001, mid_tau) * mid
    top = st(kn) + tr + md
    top = reverb(top, make_ir(key, tail[0], hf=tail[2], bright=7000), tail[1])
    return st(sub) + top


def riser(key, dur, mode='smooth', f0=110.0, octaves=2.0, tonal=0.55, rumble=0.25):
    """Build-up that ends at full intensity on the file's last sample (file end = downbeat).
    noise: resonant band-pass sweeping 280 Hz -> 9.5 kHz with rising Q + growing hiss, widening stereo;
    tonal: 5-voice band-limited supersaw gliding up `octaves`, detune + stereo spread growing, LP opening;
    'pulse': tempo-synced gate accelerating 1/8 -> 1/16 -> 1/32 (-> 1/64), aligned back from the end;
    'air': noise only (key-agnostic) with a jet-flanger shimmer."""
    rng = rng_for('riser', key)
    n = ns(dur)
    t = tax(n)
    p = t / dur
    corr = 0.75 - 0.55 * p
    c, l, r = colored(rng, n, -1.5), colored(rng, n, -1.5), colored(rng, n, -1.5)
    fc = 280.0 * (9500.0 / 280.0) ** (p ** 1.3)
    q = 0.8 + (3.2 if mode == 'air' else 2.0) * p ** 2
    nz = np.zeros((2, n))
    for ch, ind in enumerate((l, r)):
        src = np.sqrt(corr) * c + np.sqrt(1 - corr) * ind
        _, bp, hp = svf(src, fc, q)
        nz[ch] = 0.75 * unit(bp) + 0.4 * unit(hp) * p ** 1.5
    if mode == 'air':
        d = (6.0 - 5.2 * p ** 0.8) * SR / 1000.0
        for ch in range(2):
            nz[ch] = nz[ch] + 0.6 * vdelay(nz[ch], d * (1.0 + 0.15 * ch))
    layers = nz / rms(nz)
    if tonal and mode != 'air':
        f = f0 * 2.0 ** (octaves * p ** 1.6)
        det = 6.0 + 24.0 * p
        voices = np.zeros((2, n))
        for o in (-1.0, -0.5, 0.0, 0.5, 1.0):
            fv = f * 2.0 ** (o * det / 1200.0)
            ph = phase_of(fv) + rng.uniform(0, 2 * np.pi)
            voices += pan(bl_saw(ph, fv, fmax=12000.0, kmax=64), o * (0.2 + 0.7 * p))
        lpf = 450.0 * (10000.0 / 450.0) ** (p ** 1.4)
        for ch in range(2):
            voices[ch], _, _ = svf(voices[ch], lpf, 1.1)
        layers += tonal * voices / rms(voices)
    if rumble:
        lp, _, _ = svf(colored(rng, n, -6.0), 120.0, 0.8)
        layers += rumble * st(unit(lp) * p ** 1.5)
    env = db2a(-30.0 * (1.0 - p) ** 1.5)
    if mode == 'pulse':
        beats = dur / BEAT
        # (start_beat, end_beat, step_seconds) - grid counted from the file start == from the end (whole bars)
        sched = ([(0, 4, BEAT / 2), (4, 6, BEAT / 4), (6, 7, BEAT / 8), (7, 8, BEAT / 16)] if beats >= 8
                 else [(0, 2, BEAT / 2), (2, 3, BEAT / 4), (3, 4, BEAT / 8)])
        gate = np.ones(n)
        for b0, b1, step in sched:
            m = (t >= b0 * BEAT) & (t < b1 * BEAT)
            ph = ((t[m] - b0 * BEAT) % step) / step
            depth = 0.8 - 0.45 * p[m]
            gate[m] = (1 - depth) + depth * np.exp(-ph / 0.4)
        env = env * smooth(gate, 1.5)
    out = layers * env
    return reverb(out, make_ir(key, 0.8, hf=0.5, bright=11000), -16.0)


def downlifter(key, dur=1.0, tonal=0.0, f_hi=10000.0, f_lo=160.0, decay_db=46.0):
    """Falling resonant noise sweep (+ optional gliding saw pad) from a hit into calm."""
    rng = rng_for('down', key)
    n = ns(dur)
    t = tax(n)
    p = t / dur
    src = stereo_noise(rng, n, 0.35, -1.5)
    fc = f_hi * (f_lo / f_hi) ** (p ** 0.7)
    q = 1.8 - 0.9 * p
    out = np.zeros((2, n))
    for c in range(2):
        lp, bp, _ = svf(src[c], fc, q)
        out[c] = unit(bp) + 0.35 * unit(lp)
    if tonal:
        f = 660.0 * (82.0 / 660.0) ** (p ** 0.8)
        v = np.zeros((2, n))
        for o in (-1.0, 0.0, 1.0):
            fv = f * 2 ** (o * 12 / 1200)
            v += pan(bl_saw(phase_of(fv), fv, 10000.0, 48), 0.5 * o)
        for c in range(2):
            v[c], _, _ = svf(v[c], np.clip(fc * 0.6, 120, 9000), 1.2)
        out += tonal * v / rms(v) * rms(out)
    env = attack_decay(t, 0, 0.012, 1e9) * db2a(-decay_db * p ** 0.9)
    out = pan(out * env, pan_move(t, 0.0, 0.35, 0.5, 0.25))
    return reverb(out, make_ir(key, 1.3, hf=0.4, bright=9000), -11.0)


def sub_drop(key, dur=1.0, f0=120.0, f1=32.0, ftau=0.2, tau=0.3, drive=1.8, octave=0.12, air=0.0):
    """Pitch-falling sine (exp glide f0 -> f1), 8 ms click-free attack, tanh harmonics for small speakers."""
    rng = rng_for('subdrop', key)
    n = ns(dur)
    t = tax(n)
    f = f1 + (f0 - f1) * np.exp(-t / ftau)
    ph = phase_of(f)
    y = np.sin(ph) + octave * np.sin(2 * ph) * np.exp(-t / 0.12)
    y = sat(y * attack_decay(t, 0, 0.008, tau), drive)
    out = st(y)
    if air:
        a = unit(butter(stereo_noise(rng, n, 0.3), 'highpass', 3000)) * attack_decay(t, 0, 0.001, 0.012)
        out = out + air * a
    return out


def ui_click(key, f0=2100.0, tock=900.0, bright=1.0, dur=0.12):
    """Soft glassy click: inharmonic free-bar modes (1, 2.756, 5.404) + a short low 'tock' + micro noise tick."""
    rng = rng_for('click', key)
    n = ns(dur)
    t = tax(n)
    y = np.zeros(n)
    for r, tau, a in ((1.0, 0.014, 1.0), (2.756, 0.006, 0.45 * bright), (5.404, 0.0028, 0.18 * bright)):
        if f0 * r < 17000:
            y += a * np.sin(2 * np.pi * f0 * r * t) * attack_decay(t, 0, 0.0006, tau)
    y += 0.55 * np.sin(2 * np.pi * tock * t) * attack_decay(t, 0, 0.0008, 0.005)
    nz = butter(rng.standard_normal(n), 'highpass', 2500) * attack_decay(t, 0, 0.0003, 0.0015)
    y += 0.3 * nz / np.max(np.abs(nz))
    y = preroll(butter(y, 'lowpass', 11000))
    return reverb(st(y), make_ir(key, 0.25, hf=0.5, bright=9000, predelay=0.004), -20.0)


def ui_pop(key, f_start=380.0, f_end=820.0, gtau=0.016, tau=0.03, body=0.25, semis=0, dur=0.16):
    """Bubble pop: sine whose pitch rises quickly (shrinking bubble) + soft body + tiny click."""
    rng = rng_for('pop', key)
    n = ns(dur)
    t = tax(n)
    k = 2.0 ** (semis / 12.0)
    f = (f_end - (f_end - f_start) * np.exp(-t / gtau)) * k
    ph = phase_of(f)
    y = np.sin(ph) + 0.15 * np.sin(2 * ph) * np.exp(-t / 0.02)
    y *= attack_decay(t, 0, 0.0012, tau)
    y += body * np.sin(2 * np.pi * 170.0 * np.sqrt(k) * t) * attack_decay(t, 0, 0.001, 0.012)
    nz = butter(rng.standard_normal(n), 'lowpass', 6000) * attack_decay(t, 0, 0.0002, 0.0015)
    y += 0.12 * nz / np.max(np.abs(nz))
    y = sat(y, 1.2)
    return reverb(st(y), make_ir(key, 0.35, hf=0.5, bright=9000, predelay=0.006), -17.0)


def ui_tick(key, f0=3200.0, tau=0.003, noise=0.3, dur=0.04):
    """Very short tick: a damped high sine pair + band-limited noise click."""
    rng = rng_for('tick', key)
    n = ns(dur)
    t = tax(n)
    y = np.sin(2 * np.pi * f0 * t) * attack_decay(t, 0, 0.0004, tau)
    y += 0.35 * np.sin(2 * np.pi * f0 * 1.87 * t) * attack_decay(t, 0, 0.0003, tau * 0.5)
    nz = butter(rng.standard_normal(n), 'bandpass', (f0 * 0.8, min(f0 * 2.2, 16000))) * \
        attack_decay(t, 0, 0.0002, 0.0012)
    y += noise * nz / np.max(np.abs(nz))
    return st(preroll(butter(y, 'lowpass', 12000)))


KEY_STYLES = {
    # clack modes (Hz, tau s, amp), bottom-out 'thock' modes, noise click, pre-click (clicky switch), spring ping
    'thock': dict(clack=[(1250, .016, .5), (2350, .011, .8), (3600, .007, .5), (5200, .004, .3)],
                  thock=[(185, .024, 1.0), (330, .016, .55)], noise=0.35, band=(1500, 8000), pre=0.0,
                  spring=0.02, up=0.45),
    'clicky': dict(clack=[(1800, .012, .6), (3100, .008, 1.0), (4700, .005, .7), (6900, .003, .4)],
                   thock=[(240, .016, .55), (420, .012, .35)], noise=0.55, band=(2500, 11000), pre=0.007,
                   spring=0.035, up=0.6),
    'low': dict(clack=[(1600, .008, .6), (2900, .005, .7), (4400, .003, .4)], thock=[(260, .012, .5)],
                noise=0.3, band=(2000, 9000), pre=0.0, spring=0.0, up=0.4),
}


def key_hit(t, t0, rng, sty, up=False, pitch=1.0):
    """One mechanical key event at contact time t0 (down: [click-jacket snap +] clack + bottom-out thock;
    up: lighter, higher clack)."""
    n = len(t)
    tc = t0 + (sty['pre'] if not up else 0.0)  # clicky switches bottom out a few ms after the click
    tt = np.clip(t - tc, 0, None)
    y = np.zeros(n)
    jit = lambda: 1.0 + rng.uniform(-0.03, 0.03)  # noqa: E731
    for f, tau, a in sty['clack']:
        fr = f * jit() * pitch * (1.12 if up else 1.0)
        y += a * np.sin(2 * np.pi * fr * tt) * attack_decay(t, tc, 0.0003, tau * (0.8 if up else 1.0))
    if not up:
        for f, tau, a in sty['thock']:
            fr = f * jit() * pitch * (1 + 0.08 * np.exp(-tt / 0.006))
            y += a * np.sin(phase_of(fr, t, tc)) * attack_decay(t, tc, 0.0006, tau)
        if sty['pre']:
            y += 0.7 * np.sin(2 * np.pi * 5600 * pitch * np.clip(t - t0, 0, None)) * attack_decay(t, t0, 0.0002, 0.0015)
    nz = butter(rng.standard_normal(n), 'bandpass', sty['band']) * attack_decay(t, tc, 0.0002, 0.0012)
    y += sty['noise'] * nz / (np.max(np.abs(nz)) + 1e-12) * (0.6 if up else 1.0)
    if sty['spring']:
        y += sty['spring'] * np.sin(2 * np.pi * 5230 * tt) * attack_decay(t, tc, 0.001, 0.045)
    return y * (sty['up'] if up else 1.0)


def key_sound(key, style='thock', down=0.0, up=0.115, dur=0.28, pitch=1.0, gain_up=1.0):
    rng = rng_for('key', key)
    n = ns(dur)
    t = tax(n)
    sty = KEY_STYLES[style]
    y = np.zeros(n)
    if down is not None:
        y += key_hit(t, down, rng, sty, up=False, pitch=pitch)
    if up is not None:
        y += gain_up * key_hit(t, up, rng, sty, up=True, pitch=pitch)
    y = preroll(butter(y, 'lowpass', 12000))
    return reverb(st(y), make_ir(key, 0.2, hf=0.6, bright=10000, predelay=0.003), -20.0)


def chime_note(t, t0, f, tau, idx=1.1, bright=1.0):
    """Soft glass/mallet tone: 1:1 phase modulation whose index decays (bright strike -> pure sine),
    + decaying 2nd harmonic + an inharmonic 'tine' click."""
    tt = np.clip(t - t0, 0, None)
    I = idx * np.exp(-tt / 0.06)
    y = np.sin(2 * np.pi * f * tt + I * np.sin(2 * np.pi * f * tt))
    y += 0.16 * bright * np.sin(2 * np.pi * 2 * f * tt) * np.exp(-tt / (tau * 0.35))
    if 4.2 * f < 17000:
        y += 0.07 * bright * np.sin(2 * np.pi * 4.2 * f * tt) * np.exp(-tt / 0.018)
    return y * attack_decay(t, t0, 0.002, tau)


def success_chime(key, m1, m2, gap=BEAT / 4, dur=0.9, tau1=0.15, tau2=0.16):
    """Two-note chime; the 2nd note lands a 16th note (0.125 s) later, on the 120 BPM grid."""
    n = ns(dur)
    t = tax(n)
    a = pan(0.85 * chime_note(t, 0.0, midi(m1), tau1), -0.25)
    b = pan(1.0 * chime_note(t, gap, midi(m2), tau2), 0.25)
    return reverb(a + b, make_ir(key, 0.7, hf=0.55, bright=12000, predelay=0.014), -12.0)


def ding(key, m=88, dur=0.8, tau=0.17):
    n = ns(dur)
    t = tax(n)
    y = chime_note(t, 0.0, midi(m), tau, idx=1.3) + 0.25 * chime_note(t, 0.0, midi(m + 12), tau * 0.5, idx=0.6)
    return reverb(st(y), make_ir(key, 0.7, hf=0.55, bright=12000, predelay=0.014), -12.0)


def shimmer(key, notes, order='up', spacing=0.022, tau=(0.07, 0.16), twinkles=5, air=0.12, rev=(0.8, -6.0),
            dur=0.8, span=0.3):
    """Sparkle: a fast pentatonic glissando of bell grains (random pans) + twinkles + air dust + bright room."""
    rng = rng_for('shimmer', key)
    n = ns(dur)
    t = tax(n)
    seq = list(notes) if order == 'up' else list(notes)[::-1] if order == 'down' else list(rng.permutation(notes))
    out = np.zeros((2, n))
    ti = 0.0
    for i, m in enumerate(seq):
        f = midi(m) * (1 + rng.uniform(-0.002, 0.002))
        tau_i = float(np.interp(f, [1000, 8000], [tau[1], tau[0]]))
        g = np.sin(2 * np.pi * f * np.clip(t - ti, 0, None)) * attack_decay(t, ti, 0.0015, tau_i)
        if 2.76 * f < 17500:
            g += 0.2 * np.sin(2 * np.pi * 2.76 * f * np.clip(t - ti, 0, None)) * attack_decay(t, ti, 0.001, tau_i * 0.3)
        amp = 0.9 ** i * rng.uniform(0.8, 1.0) if i else 1.0
        out += amp * pan(g, rng.uniform(-0.7, 0.7) if i else 0.0)
        ti += spacing * (1 + rng.uniform(-0.25, 0.25))
    top = sorted(notes)[-4:]
    for _ in range(twinkles):
        m = rng.choice(top) + 12 * rng.integers(0, 2)
        f = min(midi(m), 9000.0)
        t0 = rng.uniform(0.06, span + 0.08)
        g = np.sin(2 * np.pi * f * np.clip(t - t0, 0, None)) * attack_decay(t, t0, 0.001, 0.05)
        out += rng.uniform(0.2, 0.35) * pan(g, rng.uniform(-0.9, 0.9))
    if air:
        a = unit(butter(stereo_noise(rng, n, 0.1), 'highpass', 7000)) * attack_decay(t, 0, 0.02, 0.12)
        out += air * a * rms(out) * 3
    return reverb(out, make_ir(key, rev[0], hf=0.7, bright=13000, predelay=0.012), rev[1])


def glitch(key, dur=0.25, flavor='bright'):
    """Digital stutter: slices of a source bank (FM tone, band noise, bass square, chirps, data bits) that are
    retriggered, bit-crushed, sample-held or gapped, with micro-fades and hard per-slice pans."""
    rng = rng_for('glitch', key)
    n = ns(dur)
    t = tax(n)
    bits = np.repeat(rng.choice([-1.0, 1.0], n // 24 + 1), 24)[:n]
    chirp_f = 4000.0 * (300.0 / 4000.0) ** ((t % 0.04) / 0.04)
    src = {
        'noise': unit(butter(rng.standard_normal(n), 'bandpass', (900, 7000))) / 3.0,
        'fm': np.sin(2 * np.pi * 1350 * t + 3.0 * np.sin(2 * np.pi * 415 * t)),
        'bass': bl_square(2 * np.pi * 110 * t, 110.0) * 1.1,
        'chirp': np.sin(phase_of(chirp_f)),
        'data': butter(bits, 'lowpass', 7000) * 0.8,
    }
    w = {'bright': [0.30, 0.35, 0.0, 0.15, 0.20],
         'dark': [0.20, 0.15, 0.45, 0.0, 0.20],
         'data': [0.15, 0.25, 0.0, 0.30, 0.30]}[flavor]
    names = list(src)
    out = np.zeros((2, n))
    mf = ns(0.0004)
    ramp = 0.5 - 0.5 * np.cos(np.pi * np.arange(mf) / mf)
    pos, first = 0, True
    end = n - ns(0.06)
    while pos < end:
        seg = min(ns(rng.choice([0.008, 0.012, 0.016, 0.024, 0.032], p=[.15, .25, .25, .2, .15])), end - pos)
        if seg < 2 * mf + 8:
            break
        mode = 'plain' if first else rng.choice(['plain', 'repeat', 'crush', 'hold', 'gap'], p=[.2, .3, .2, .18, .12])
        s = ('fm' if flavor != 'dark' else 'bass') if first else rng.choice(names, p=w)
        a0 = int(rng.integers(0, n - seg))
        g = src[s][a0:a0 + seg].copy()
        if mode == 'repeat':
            gl = max(ns(0.003), seg // int(rng.integers(2, 5)))
            u = src[s][a0:a0 + gl].copy()
            u[:mf] *= ramp
            u[-mf:] *= ramp[::-1]
            g = np.tile(u, seg // gl + 1)[:seg]
        elif mode == 'crush':
            q = 2 ** (int(rng.integers(3, 7)) - 1)
            pk = np.max(np.abs(g)) + 1e-9
            g = 0.5 * g + 0.5 * np.round(g / pk * q) / q * pk
        elif mode == 'hold':
            h = int(rng.integers(4, 16))
            g = np.repeat(g[::h], h)[:seg]
        elif mode == 'gap':
            g *= 0.0
        g[:mf] *= ramp
        g[-mf:] *= ramp[::-1]
        amp = 1.0 if first else rng.uniform(0.45, 0.95)
        pp = 0.0 if first else rng.choice([-0.7, -0.35, 0.0, 0.35, 0.7])
        out[:, pos:pos + seg] += amp * pan(g, pp)
        pos += seg
        first = False
    out *= 0.35 + 0.65 * np.exp(-t / 0.12)
    out = butter(butter(out, 'lowpass', 12500), 'highpass', 120)
    return reverb(out, make_ir(key, 0.25, hf=0.5, bright=10000, predelay=0.005), -20.0)


def reverse_swell(key, dur):
    """Reverse-cymbal style swell: a bright noise 'crash' + low body through a long room, then time-reversed,
    so it grows to full level on the file's last sample (file end = drop)."""
    rng = rng_for('rswell', key)
    n = ns(dur)
    t = tax(n)
    crash = unit(butter(stereo_noise(rng, n, 0.3, -1.0), 'bandpass', (1800, 12000)))
    crash *= attack_decay(t, 0, 0.001, dur * 0.2)
    body = unit(butter(stereo_noise(rng, n, 0.6, -4.0), 'lowpass', 350)) * attack_decay(t, 0, 0.002, dur * 0.1) * 0.5
    x = crash + body
    x = reverb(x, make_ir(key, dur * 0.9, hf=0.55, bright=11000, length=dur), 0.0, hp=120)
    return x[:, ::-1].copy()


def sweep(key, notes=(84, 86, 88, 91, 93, 96, 98), spacing=0.045, dur=0.6, air=0.5, rev=-12.0):
    """Group reveal (card grid / logo row): a rising tick cluster of glassy pings panned L->R over a soft air
    sweep. hit = first tick = group onset."""
    rng = rng_for('sweep', key)
    n = ns(dur)
    t = tax(n)
    out = np.zeros((2, n))
    k = len(notes)
    for i, m in enumerate(notes):
        t0 = i * spacing
        f = midi(m)
        g = np.sin(2 * np.pi * f * np.clip(t - t0, 0, None)) * attack_decay(t, t0, 0.0008, 0.02)
        g += 0.3 * np.sin(2 * np.pi * 2.76 * f * np.clip(t - t0, 0, None)) * attack_decay(t, t0, 0.0006, 0.006)
        amp = (1.0 if i == 0 else 0.62 + 0.25 * np.sin(np.pi * i / k))
        out += amp * pan(g, -0.6 + 1.2 * i / max(k - 1, 1))
    tp = spacing * (k - 1) * 0.6
    env = bell(t, tp, 0.08, 0.06, 2.0, 2.6)
    src = stereo_noise(rng, n, 0.3, -1.5)
    fc = curve(t, [(0, 1800), (tp, 7000), (dur, 9000)], log=True)
    a = np.zeros((2, n))
    for c in range(2):
        _, bp, _ = svf(src[c], fc, 1.4)
        a[c] = unit(bp) * env
    out += air * a * rms(out) / max(rms(a), 1e-9)
    return reverb(out, make_ir(key, 0.6, hf=0.6, bright=12000, predelay=0.01), rev)


def bloop(key, f_start=160.0, f_end=420.0, dur=0.4, servo=0.18):
    """Mascot landing: round low pop + landing thump + a tiny servo whirr (geared, rising)."""
    rng = rng_for('bloop', key)
    n = ns(dur)
    t = tax(n)
    f = f_end - (f_end - f_start) * np.exp(-t / 0.03)
    ph = phase_of(f)
    y = (np.sin(ph) + 0.2 * np.sin(2 * ph) * np.exp(-t / 0.04)) * attack_decay(t, 0, 0.0015, 0.09)
    y += 0.4 * np.sin(2 * np.pi * 90 * t) * attack_decay(t, 0, 0.002, 0.03)
    out = st(sat(y, 1.3))
    if servo:
        t0, ln = 0.035, 0.16
        fs = curve(t, [(0, 480), (t0, 480), (t0 + ln, 760), (dur, 760)], log=True) * (1 + rng.uniform(-.02, .02))
        s = bl_saw(phase_of(fs), fs, 6000.0, 16) * (0.6 + 0.4 * np.sin(2 * np.pi * 58 * t))
        _, s, _ = svf(s, 1400.0, 1.2)
        se = np.clip((t - t0) / 0.015, 0, 1) * np.clip((t0 + ln - t) / 0.05, 0, 1)
        s = s * (0.5 - 0.5 * np.cos(np.pi * se))
        out += servo * pan(s / (np.max(np.abs(s)) + 1e-9), 0.2) * np.max(np.abs(y))
    return reverb(out, make_ir(key, 0.35, hf=0.5, bright=9000, predelay=0.006), -16.0)


# ============================================================================ registry

REG: list[dict] = []


def reg(name, var, fn, *, hit, use, level, dur, fades=(0.003, 0.010), eob=False, character='', pitch=None,
        checks=()):
    REG.append(dict(name=name, var=var, fn=fn, hit=hit, use=use, level=level, dur=dur, fades=fades, eob=eob,
                    character=character, pitch=pitch, checks=checks, file=f'{name}_{var}.wav'))


# ---- whooshes -------------------------------------------------------------------------------
W_IN = [
    ('airy rise, drifts in from the left', dict(
        dur=0.40, tp=0.29, w_pre=0.075, w_post=0.027, p_pre=2.2, p_post=3.8,
        fc=[(0, 380), (0.29, 4800), (0.40, 6200)], q=[(0, 1.0), (0.29, 2.0), (0.40, 1.4)],
        pan_=(-0.6, 0.1, 0.22, 0.08), air=0.45, body=0.35, whistle=0.12, rev=(0.3, -19))),
    ('bright and fast, drifts in from the right (zoom-through)', dict(
        dur=0.36, tp=0.27, w_pre=0.060, w_post=0.023, p_pre=2.2, p_post=3.8,
        fc=[(0, 600), (0.27, 7500), (0.36, 9000)], q=[(0, 1.2), (0.27, 2.4), (0.36, 1.6)],
        pan_=(0.55, -0.1, 0.20, 0.07), air=0.6, body=0.25, whistle=0.18, rev=(0.28, -19))),
    ('soft and warm, near-centre (gentle push-in)', dict(
        dur=0.44, tp=0.31, w_pre=0.090, w_post=0.032, p_pre=2.0, p_post=3.6,
        fc=[(0, 220), (0.31, 2600), (0.44, 3200)], q=[(0, 0.8), (0.31, 1.3), (0.44, 1.0)],
        pan_=(-0.25, 0.05, 0.25, 0.10), air=0.18, body=0.55, slope=-3.0, rev=(0.32, -18))),
]
for i, (ch, kw) in enumerate(W_IN, 1):
    reg('whoosh_in', i, lambda key, kw=kw: whoosh(key, **kw), hit='env', level=-8, dur=(0.35, 0.45),
        fades=(0.025, 0.020), character=ch,
        checks=(('hit_between', 0.55, 0.85), ('rises', 'centroid', 0.5), ('rises', 'level', 0.8, 'pre')),
        use='Element/scene flying in, zoom-through, whip into a new scene. Rising air; put hit_offset (the '
            'loudest point) on the cut / arrival frame.')

W_OUT = [
    ('airy fall, exits to the right', dict(
        dur=0.40, tp=0.085, w_pre=0.028, w_post=0.075, p_pre=2.6, p_post=2.4,
        fc=[(0, 5600), (0.085, 4600), (0.40, 320)], q=[(0, 1.8), (0.085, 2.0), (0.40, 0.9)],
        pan_=(0.0, 0.6, 0.20, 0.09), air=0.45, body=0.35, whistle=0.1, rev=(0.6, -15))),
    ('bright fall, exits to the left', dict(
        dur=0.36, tp=0.070, w_pre=0.024, w_post=0.065, p_pre=2.6, p_post=2.4,
        fc=[(0, 8000), (0.07, 7000), (0.36, 450)], q=[(0, 2.0), (0.07, 2.3), (0.36, 1.0)],
        pan_=(0.0, -0.6, 0.18, 0.08), air=0.6, body=0.25, whistle=0.15, rev=(0.5, -16))),
    ('soft warm fall, near-centre', dict(
        dur=0.44, tp=0.100, w_pre=0.034, w_post=0.075, p_pre=2.4, p_post=2.8,
        fc=[(0, 3200), (0.10, 2600), (0.44, 200)], q=[(0, 1.2), (0.10, 1.3), (0.44, 0.8)],
        pan_=(0.05, 0.3, 0.22, 0.10), air=0.18, body=0.55, slope=-3.0, rev=(0.5, -14))),
]
for i, (ch, kw) in enumerate(W_OUT, 1):
    reg('whoosh_out', i, lambda key, kw=kw: whoosh(key, **kw), hit='env', level=-10, dur=(0.35, 0.45),
        fades=(0.010, 0.040), character=ch,
        checks=(('hit_between', 0.10, 0.35), ('falls', 'centroid', 0.5), ('falls', 'level', 0.8, 'post')),
        use='Element/scene leaving, pull-back, card dismissed. Falling air; start it with the move '
            '(hit_offset is the early peak).')

WHIP = [
    ('L->R pass-by', dict(dur=0.18, tp=0.085, w_pre=0.022, w_post=0.016, p_pre=2.6, p_post=3.6,
                          fc=[(0, 900), (0.085, 5500), (0.18, 1500)], q=[(0, 1.0), (0.085, 1.8), (0.18, 1.1)],
                          pan_=(-0.85, 0.85, 0.085, 0.03), air=0.55, body=0.3, whistle=0.1, rev=(0.25, -17))),
    ('R->L, tighter and brighter', dict(dur=0.15, tp=0.068, w_pre=0.018, w_post=0.013, p_pre=2.6, p_post=3.6,
                                        fc=[(0, 1200), (0.068, 7500), (0.15, 2200)],
                                        q=[(0, 1.1), (0.068, 2.0), (0.15, 1.2)],
                                        pan_=(0.85, -0.85, 0.068, 0.022), air=0.7, body=0.2, whistle=0.12,
                                        rev=(0.22, -18))),
    ('L->R, darker with jet body', dict(dur=0.22, tp=0.11, w_pre=0.028, w_post=0.020, p_pre=2.4, p_post=3.6,
                                        fc=[(0, 500), (0.11, 3800), (0.22, 900)],
                                        q=[(0, 0.9), (0.11, 1.5), (0.22, 1.0)],
                                        pan_=(-0.7, 0.7, 0.11, 0.04), air=0.35, body=0.5, slope=-3.0,
                                        flange=0.4, flange_ms=(5.0, 0.6), rev=(0.28, -16))),
]
for i, (ch, kw) in enumerate(WHIP, 1):
    reg('whip', i, lambda key, kw=kw: whoosh(key, **kw), hit='env', level=-8, dur=(0.15, 0.22),
        fades=(0.010, 0.020), character=ch, checks=(('hit_between', 0.35, 0.60),),
        use='Whip-pan / swipe transitions between scenes. Very fast pass-by; hit_offset (loudest point) on the '
            'cut frame.')

SWOOSH = [
    ('big L->R camera move with jet flange', dict(
        dur=0.80, tp=0.50, w_pre=0.16, w_post=0.09, p_pre=2.0, p_post=3.2,
        fc=[(0, 220), (0.50, 3200), (0.80, 900)], q=[(0, 0.9), (0.50, 1.6), (0.80, 1.0)],
        pan_=(-0.7, 0.7, 0.50, 0.15), flange=0.55, flange_ms=(8.0, 0.7), body=0.5, rumble=0.35, air=0.35,
        whistle=0.1, rev=(0.7, -14))),
    ('R->L sweep, earlier peak, brighter jet', dict(
        dur=0.80, tp=0.40, w_pre=0.12, w_post=0.11, p_pre=2.0, p_post=3.0,
        fc=[(0, 300), (0.40, 4500), (0.80, 700)], q=[(0, 1.0), (0.40, 1.8), (0.80, 1.0)],
        pan_=(0.6, -0.6, 0.40, 0.13), flange=0.75, flange_ms=(7.0, 0.5), body=0.4, rumble=0.2, air=0.5,
        whistle=0.12, rev=(0.7, -14))),
    ('deep push-in (tilt-to-flat / tabletop), late peak, sub rumble', dict(
        dur=0.80, tp=0.58, w_pre=0.20, w_post=0.06, p_pre=2.2, p_post=3.8,
        fc=[(0, 120), (0.58, 2200), (0.80, 1400)], q=[(0, 0.8), (0.58, 1.4), (0.80, 1.0)],
        pan_=(-0.3, 0.2, 0.53, 0.18), flange=0.3, flange_ms=(9.0, 1.0), body=0.7, rumble=0.6, air=0.25,
        slope=-3.0, rev=(0.6, -17))),
]
for i, (ch, kw) in enumerate(SWOOSH, 1):
    reg('swoosh_long', i, lambda key, kw=kw: whoosh(key, **kw), hit='env', level=-8, dur=(0.78, 0.82),
        fades=(0.060, 0.060), character=ch, checks=(('hit_between', 0.40, 0.80),),
        use='Big camera moves: hero tilt-to-flat, 3D wall of screens, long zoom. Start ~6 f before the move; '
            'hit_offset on the fastest frame of the move.')

# ---- impacts --------------------------------------------------------------------------------
DEEP = [
    ('50 Hz sub, balanced', dict(f_sub=50, f_start=140, pitch_tau=0.028, sub_tau=0.26, drive=1.8,
                                 knock=((170, 0.06, 0.6), (310, 0.03, 0.3)), trans=0.28, trans_tau=0.007,
                                 trans_band=(1500, 9000), mid=0.25, mid_tau=0.05, mid_band=(300, 2500),
                                 tail=(1.1, -12, 0.35))),
    ('45 Hz sub, bigger and darker (logo drop)', dict(f_sub=45, f_start=120, pitch_tau=0.035, sub_tau=0.30,
                                                      drive=1.5, knock=((150, 0.07, 0.55), (280, 0.035, 0.2)), trans=0.2,
                                                      trans_tau=0.009, trans_band=(1000, 7000), mid=0.2,
                                                      mid_tau=0.06, mid_band=(250, 2000), tail=(1.4, -11, 0.3))),
    ('55 Hz sub, tighter and punchier', dict(f_sub=55, f_start=160, pitch_tau=0.022, sub_tau=0.20, drive=2.2,
                                             knock=((200, 0.05, 0.65), (420, 0.025, 0.3)), trans=0.33,
                                             trans_tau=0.006, trans_band=(2000, 11000), mid=0.3, mid_tau=0.045,
                                             mid_band=(350, 3000), tail=(0.9, -13, 0.4))),
]
for i, (ch, kw) in enumerate(DEEP, 1):
    reg('impact_deep', i, lambda key, kw=kw: impact(key, 1.2, **kw), hit='onset', level=2, dur=(1.1, 1.3),
        fades=(0.003, 0.150), character=ch, checks=(('sub_fraction', 0.25),),
        use='Brand / logo reveal on the drop, end-card final hit. Stack with sub_drop; duck the bed -6 dB.')

SOFT = [
    ('75 Hz thump + wood knock, neutral', dict(f_sub=75, f_start=170, pitch_tau=0.018, sub_tau=0.085, drive=1.6,
                                               knock=((230, 0.04, 0.5), (460, 0.02, 0.25)), trans=0.25,
                                               trans_tau=0.005, trans_band=(1800, 8000), mid=0.3, mid_tau=0.03,
                                               mid_band=(400, 3000), tail=(0.6, -14, 0.4))),
    ('85 Hz, brighter and tighter', dict(f_sub=85, f_start=200, pitch_tau=0.015, sub_tau=0.07, drive=1.8,
                                         knock=((260, 0.035, 0.55), (620, 0.018, 0.3)), trans=0.33,
                                         trans_tau=0.004, trans_band=(2500, 11000), mid=0.35, mid_tau=0.028,
                                         mid_band=(500, 3500), tail=(0.5, -15, 0.45))),
    ('65 Hz, rounder and softer', dict(f_sub=65, f_start=150, pitch_tau=0.02, sub_tau=0.11, drive=1.4,
                                       knock=((190, 0.05, 0.4),), trans=0.18, trans_tau=0.006,
                                       trans_band=(1200, 6000), mid=0.25, mid_tau=0.04, mid_band=(300, 2500),
                                       tail=(0.7, -13, 0.35))),
]
for i, (ch, kw) in enumerate(SOFT, 1):
    reg('impact_soft', i, lambda key, kw=kw: impact(key, 0.6, **kw), hit='onset', level=0, dur=(0.5, 0.7),
        fades=(0.003, 0.080), character=ch, checks=(('sub_fraction', 0.10),),
        use='Kinetic word slams / title slams (S02): transient on the contact frame; duck the bed -5 dB.')

# ---- risers / downlifters / sub ---------------------------------------------------------------
RISER_V = [('smooth: resonant noise sweep + gliding supersaw (A2->A4) + rumble', dict(mode='smooth')),
           ('pulsed: tempo-synced gate accelerating 1/8 -> 1/16 -> 1/32 (-> 1/64 on 2 bars)',
            dict(mode='pulse', tonal=0.4)),
           ('air: noise-only (key-agnostic) with jet shimmer', dict(mode='air', tonal=0.0, rumble=0.2))]
for name, bars in (('riser_1bar', 1), ('riser_2bar', 2)):
    for i, (ch, kw) in enumerate(RISER_V, 1):
        reg(name, i, lambda key, kw=kw, bars=bars: riser(key, bars * BAR, **kw), hit='end', level=-6,
            dur=('exact', bars * BAR), fades=(0.005, 0.003), eob=True, character=ch,
            checks=(('ends_loudest',), ('no_presilence',), ('rises', 'centroid', 0.8), ('rises', 'level', 0.8)),
            pitch='tonal glide ends on A4 (440 Hz)' if kw.get('mode') != 'air' else None,
            use=f'{bars}-bar build into a drop / reveal. The last sample is the downbeat: '
                f'at = dropFrame - {bars * 60}. Leave 8-15 f of silence in the bed before the drop.')

for i, (ch, kw) in enumerate([('noise-only, airy', dict(tonal=0.0, f_hi=11000.0)),
                              ('noise + falling saw pad', dict(tonal=0.5, f_hi=9000.0))], 1):
    reg('downlifter', i, lambda key, kw=kw: downlifter(key, 1.0, **kw), hit='onset', level=-12,
        dur=(0.98, 1.02), fades=(0.004, 0.080), character=ch,
        checks=(('falls', 'centroid', 0.8), ('falls', 'level', 0.8)),
        use='Release after a peak: the moment after the drop / hit settles into the next calmer section.')

SUB = [('120 -> 32 Hz', dict(f0=120, f1=32, ftau=0.20, tau=0.22, drive=1.8)),
       ('90 -> 30 Hz, deeper and longer', dict(f0=90, f1=30, ftau=0.28, tau=0.26, drive=1.5)),
       ('150 -> 38 Hz, punchier with air tick', dict(f0=150, f1=38, ftau=0.15, tau=0.18, drive=2.2, air=0.12))]
for i, (ch, kw) in enumerate(SUB, 1):
    reg('sub_drop', i, lambda key, kw=kw: sub_drop(key, 1.0, **kw), hit='onset', level=0, dur=(0.98, 1.02),
        fades=(0.004, 0.120), character=ch, checks=(('sub_fraction', 0.6), ('falls', 'centroid', 0.7)),
        use='Sub boom under the logo drop / big reveal; stack with impact_deep on the same frame.')

# ---- UI -------------------------------------------------------------------------------------
for i, (ch, kw) in enumerate([('glassy 2.1 kHz', dict(f0=2100, tock=900)),
                              ('brighter 2.7 kHz', dict(f0=2700, tock=1100, bright=1.1)),
                              ('softer 1.5 kHz', dict(f0=1500, tock=700, bright=0.6))], 1):
    reg('ui_click', i, lambda key, kw=kw: ui_click(key, **kw), hit='onset', level=-12, dur=(0.05, 0.15),
        fades=(0.002, 0.010), character=ch, checks=(('short', 0.03),),
        use='Cursor clicks, button presses, toggles, CTA click (S13, S22).')

POPS = [('bubble 380->820 Hz', dict(f_start=380, f_end=820)),
        ('snappier 450->1000 Hz', dict(f_start=450, f_end=1000, gtau=0.012, tau=0.025)),
        ('rounder 300->650 Hz', dict(f_start=300, f_end=650, gtau=0.02, tau=0.038, body=0.3))]
for i, (ch, kw) in enumerate(POPS, 1):
    reg('ui_pop', i, lambda key, kw=kw: ui_pop(key, **kw), hit='onset', level=-14, dur=(0.1, 0.25),
        fades=(0.002, 0.010), character=ch, checks=(('short', 0.08),),
        use='Cards / toasts / notifications / chips appearing (S06, S16) on their first frame.')

for i, (ch, kw) in enumerate([('3.2 kHz', dict(f0=3200, tau=0.003)), ('2.4 kHz, softer', dict(f0=2400, tau=0.0035)),
                              ('4.3 kHz, crisper', dict(f0=4300, tau=0.0025, noise=0.4))], 1):
    reg('ui_tick', i, lambda key, kw=kw: ui_tick(key, **kw), hit='onset', level=-22, dur=(0.02, 0.06),
        fades=(0.002, 0.005), character=ch, checks=(('short', 0.012),),
        use='Counters ticking, cursor/typing micro-feedback, tiny state flips. Keep >= 3 frames apart.')

for i, (ch, sty) in enumerate([('tactile "thocky" switch', 'thock'), ('clicky switch (click jacket + clack)', 'clicky'),
                               ('low-profile / laptop key', 'low')], 1):
    reg('key_press', i, lambda key, sty=sty: key_sound(key, sty, 0.0, 0.115, 0.28), hit='onset', level=-10,
        dur=(0.2, 0.35), fades=(0.002, 0.010), character=ch + '; down at 0.002 s, up at 0.117 s (~3.5 f later)',
        use='Keycap press close-up (S14) / "press 1" moments: down on contact frame, release sound ~3.5 f later.')
    reg('key_down', i, lambda key, sty=sty: key_sound(key, sty, 0.0, None, 0.12), hit='onset', level=-10,
        dur=(0.08, 0.15), fades=(0.002, 0.010), character=ch + ', press only',
        use='Keycap down on the contact frame (pair with key_up on release).')
    reg('key_up', i, lambda key, sty=sty: key_sound(key, sty, None, 0.0, 0.10), hit='onset', level=-18,
        dur=(0.06, 0.15), fades=(0.002, 0.010), character=ch + ', release only',
        use='Keycap release (lighter, higher).')

for i, pitch in enumerate((0.94, 1.0, 1.06, 1.12), 1):
    reg('type', i, lambda key, pitch=pitch: key_sound(key, 'low', 0.0, None, 0.1, pitch=pitch), hit='onset',
        level=-20, dur=(0.05, 0.12), fades=(0.002, 0.010), character=f'soft typing key, pitch x{pitch}',
        use='Typing into a field (S15): rotate type_1..4 every 2-3 characters.')

for s in range(1, 6):
    reg('ui_pop_up', s, lambda key, s=s: ui_pop(key, 380, 820, semis=s), hit='onset', level=-14,
        dur=(0.1, 0.25), fades=(0.002, 0.010), character=f'ui_pop_1 +{s} semitone(s)', pitch=f'+{s} st',
        use='Stacked notification pop-ins (S06): each successive toast one semitone higher '
            '(ui_pop_1, then ui_pop_up_1..5).')

SPARK = [('ascending pentatonic sparkle G6->E8', dict(notes=[91, 93, 96, 98, 100, 103, 105, 108, 110, 112])),
         ('random twinkle cluster, denser', dict(notes=[96, 98, 100, 103, 105, 108, 110, 112], order='random',
                                                 spacing=0.014, tau=(0.05, 0.11), twinkles=7)),
         ('descending, lower "magic" C6-E7', dict(notes=[84, 86, 88, 91, 93, 96, 98, 100], order='down',
                                                  spacing=0.028, tau=(0.055, 0.10), twinkles=3, rev=(0.7, -9.0)))]
for i, (ch, kw) in enumerate(SPARK, 1):
    reg('shimmer', i, lambda key, kw=kw: shimmer(key, **kw), hit='onset', level=-14, dur=(0.75, 0.85),
        fades=(0.002, 0.220), character=ch, pitch='C major pentatonic',
        use='Success / confirm / glint across the logo (S08) / counter lands. Onset on the event (C+3 f).')

CHIMES = [('major third C6 -> E6', (84, 88)), ('perfect fifth C6 -> G6', (84, 91)), ('perfect fifth G5 -> D6', (79, 86))]
for i, (ch, (m1, m2)) in enumerate(CHIMES, 1):
    reg('success_chime', i, lambda key, m1=m1, m2=m2: success_chime(key, m1, m2), hit='onset', level=-14,
        dur=(0.85, 0.95), fades=(0.002, 0.220), character=ch + '; 2nd note a 16th (0.125 s) later',
        pitch=f'{m1}->{m2} (MIDI)',
        use='Task done / UI success state (S13) / onboarding complete. First note on the event frame.')

for i, (ch, fl) in enumerate([('bright digital stutter', 'bright'), ('darker bass stutter', 'dark'),
                              ('data / chirp stutter', 'data')], 1):
    reg('glitch', i, lambda key, fl=fl: glitch(key, 0.25, fl), hit='onset', level=-16, dur=(0.24, 0.26),
        fades=(0.002, 0.010), character=ch,
        use='Flash-cut chaos montage (S05) - one per cut; glitchy transitions and error states.')

# ---- extras named in brief/style.md 7.5 ----------------------------------------------------------
for i, d in enumerate((1.0, 2.0), 1):
    reg('reverse_swell', i, lambda key, d=d: reverse_swell(key, d), hit='end', level=-6, dur=('exact', d),
        fades=(0.010, 0.003), eob=True, character=f'reversed crash + room, {d:g} s ({int(d / BEAT)} beats)',
        checks=(('ends_loudest',), ('rises', 'level', 0.8)),
        use=f'Swell INTO the logo drop / reveal; last sample = drop frame: at = dropFrame - {int(d * FPS)}.')

for i, (ch, m) in enumerate([('E6 bell ding', 88), ('C6 bell ding', 84)], 1):
    reg('ding', i, lambda key, m=m: ding(key, m), hit='onset', level=-12, dur=(0.75, 0.85),
        fades=(0.002, 0.220), character=ch, pitch=f'MIDI {m}',
        use='Counter lands (S07) / single confirm.')

for i, (ch, kw) in enumerate([('round pop + landing thump + servo whirr', dict()),
                              ('lower bloop, no servo', dict(f_start=130, f_end=330, servo=0.0))], 1):
    reg('bloop', i, lambda key, kw=kw: bloop(key, **kw), hit='onset', level=-12, dur=(0.3, 0.45),
        fades=(0.002, 0.030), character=ch, use='UBI mascot lands / appears (S19) on the land frame.')

for i, (ch, kw) in enumerate([('rising tick cluster C6->D7 over air sweep, L->R', dict()),
                              ('tighter cluster, less air', dict(notes=(88, 91, 93, 96, 100), spacing=0.035,
                                                                 air=0.3))], 1):
    reg('sweep', i, lambda key, kw=kw: sweep(key, **kw), hit='onset', level=-16, dur=(0.55, 0.65),
        fades=(0.002, 0.060), character=ch, pitch='C major pentatonic',
        use='ONE sound for a whole group reveal: card grid pop (S16), logo row (S20). Onset = group onset.')

# style.md 7.5 names -> primary files (byte copies)
ALIASES = {
    'impact.wav': 'impact_soft_1.wav', 'sub-boom.wav': 'sub_drop_1.wav', 'reverse-swell.wav': 'reverse_swell_1.wav',
    'riser-2bar.wav': 'riser_2bar_1.wav', 'whoosh-fast.wav': 'whoosh_in_2.wav', 'whoosh-soft.wav': 'whoosh_in_3.wav',
    'click.wav': 'ui_click_1.wav', 'key-down.wav': 'key_down_1.wav', 'key-up.wav': 'key_up_1.wav',
    'type-1.wav': 'type_1.wav', 'type-2.wav': 'type_2.wav', 'type-3.wav': 'type_3.wav', 'type-4.wav': 'type_4.wav',
    'pop.wav': 'ui_pop_1.wav', 'pop+1.wav': 'ui_pop_up_1.wav', 'pop+2.wav': 'ui_pop_up_2.wav',
    'pop+3.wav': 'ui_pop_up_3.wav', 'pop+4.wav': 'ui_pop_up_4.wav', 'pop+5.wav': 'ui_pop_up_5.wav',
    'tick.wav': 'ui_tick_1.wav', 'glitch.wav': 'glitch_1.wav', 'shimmer.wav': 'shimmer_1.wav',
    'ding.wav': 'ding_1.wav', 'bloop.wav': 'bloop_1.wav', 'sweep.wav': 'sweep_1.wav',
}

# ============================================================================ analysis / validation


def load(path: Path):
    sr, d = wavfile.read(str(path))
    return sr, d


def env_rms(mono, ms):
    return np.sqrt(np.maximum(smooth(mono ** 2, ms), 0))


def trend(m, what='centroid', win=0.02, floor_db=30.0, part=None):
    """Rank correlation between time and the per-window spectral centroid (or level in dB), using only windows
    within floor_db of the loudest one. +1 = steadily rising, -1 = steadily falling."""
    w = ns(win)
    k = m['n'] // w
    frames = m['mono'][:k * w].reshape(k, w) * np.hanning(w)
    P = np.abs(np.fft.rfft(frames, axis=1)) ** 2
    f = np.fft.rfftfreq(w, 1 / SR)
    lvl = 10 * np.log10(P.sum(axis=1) + 1e-20)
    keep = lvl > lvl.max() - floor_db
    tc = (np.arange(k) + 0.5) * win
    if part == 'pre':
        keep &= tc <= m['hit']
    elif part == 'post':
        keep &= tc >= m['hit']
    val = (P * f).sum(axis=1) / (P.sum(axis=1) + 1e-20) if what == 'centroid' else lvl
    idx = np.nonzero(keep)[0]
    if len(idx) < 4:
        return 0.0
    r = lambda a: np.argsort(np.argsort(a))  # noqa: E731
    return float(np.corrcoef(r(idx), r(val[idx]))[0, 1])


def measure(e, path: Path):
    sr, d = load(path)
    x = d.astype(np.float64) / 32768.0
    if x.ndim == 1:
        x = x[:, None]
    n = x.shape[0]
    mono = x.mean(axis=1)
    pw = (x ** 2).mean(axis=1)
    absmax = np.abs(x).max(axis=1)
    peak = float(absmax.max())
    if e['hit'] == 'end':
        hit = n / sr
    elif e['hit'] == 'env':
        hit = int(np.argmax(smooth(pw, 10.0))) / sr
    elif e['hit'] == 'peak':
        hit = int(np.argmax(absmax)) / sr
    else:  # onset: first time the 1 ms envelope reaches 25 % of its maximum
        a = smooth(absmax, 1.0)
        hit = int(np.argmax(a >= 0.25 * a.max())) / sr
    win = max(1, int(0.05 * sr))
    k = n // win
    seg_rms = np.sqrt((x[:k * win] ** 2).mean(axis=1).reshape(k, win).mean(axis=1)) if k else np.array([rms(x)])
    spec = np.abs(np.fft.rfft(mono)) ** 2
    fr = np.fft.rfftfreq(n, 1 / sr)
    tp = float(np.abs(signal.resample_poly(x, 4, 1, axis=0)).max())
    lr = float(np.corrcoef(x[:, 0], x[:, -1])[0, 1]) if x.shape[1] == 2 and x[:, 0].std() > 0 else 1.0
    return dict(sr=sr, ch=x.shape[1], n=n, dur=n / sr, dtype=str(d.dtype), x=x, mono=mono, tp_dbtp=a2db(tp), lr=lr,
                peak_dbfs=a2db(peak), rms_dbfs=a2db(rms(x)), max_rms_dbfs=a2db(float(seg_rms.max())),
                dc=float(np.abs(x.mean(axis=0)).max()), hit=hit,
                sub_frac=float(spec[fr < 120].sum() / spec.sum()),
                centroid=float((fr * spec).sum() / spec.sum()), first=int(np.abs(d[0]).max()),
                last=int(np.abs(d[-1]).max()))


def validate(e, m):
    errs = []
    if m['sr'] != SR:
        errs.append(f"sample rate {m['sr']}")
    if m['ch'] != 2:
        errs.append(f"channels {m['ch']}")
    if m['dtype'] != 'int16':
        errs.append(f"dtype {m['dtype']}")
    kind = e['dur']
    if kind[0] == 'exact':
        if m['n'] != ns(kind[1]):
            errs.append(f"length {m['n']} samples != {ns(kind[1])}")
    elif not (kind[0] - 1e-9 <= m['dur'] <= kind[1] + 1e-9):
        errs.append(f"duration {m['dur']:.3f} outside {kind}")
    if m['peak_dbfs'] > -0.9 or m['peak_dbfs'] < -2.5:
        errs.append(f"peak {m['peak_dbfs']:.2f} dBFS")
    if m['tp_dbtp'] > TP_CEIL + 0.05:
        errs.append(f"true peak {m['tp_dbtp']:.2f} dBTP")
    if m['dc'] > 1e-3:
        errs.append(f"DC {m['dc']:.2e}")
    x = m['x']
    pk = np.abs(x).max()
    head = np.abs(x[:ns(0.0004)]).max()
    tail = np.abs(x[-ns(0.0004):]).max()
    if m['first'] > 2 or head > 0.5 * pk:
        errs.append(f'no fade-in (first={m["first"]} LSB, first 0.4 ms {head / pk:.2f} of peak)')
    if m['last'] > 2 or tail > 0.5 * pk:
        errs.append(f'no fade-out (last={m["last"]} LSB, last 0.4 ms {tail / pk:.2f} of peak)')
    # lean files: no long dead tail (except exact-length risers)
    if kind[0] != 'exact':
        e10 = env_rms(m['mono'], 10.0)
        alive = np.nonzero(e10 > db2a(-66))[0]
        dead = (m['n'] - (alive[-1] if len(alive) else 0)) / SR
        if dead > 0.25:
            errs.append(f'{dead:.2f} s of trailing silence')
    # natural decay: the 20 ms just before the fade-out must sit >= 24 dB under the loudest 20 ms
    if not e['eob'] and m['dur'] >= 0.1:
        w, fo = ns(0.02), ns(e['fades'][1])
        p = (x ** 2).mean(axis=1)
        k = len(p) // w
        loud = np.sqrt(p[:k * w].reshape(k, w).mean(axis=1).max())
        pre = a2db(np.sqrt(p[len(p) - fo - w:len(p) - fo].mean()) / loud)
        m['tail_db'] = pre
        if pre > -24:
            errs.append(f'tail still at {pre:.1f} dB before the fade-out (truncated decay)')
    for c in e['checks']:
        if c[0] == 'hit_between':
            r = m['hit'] / m['dur']
            if not (c[1] <= r <= c[2]):
                errs.append(f'hit at {r:.2f} of duration, expected {c[1]}..{c[2]}')
        elif c[0] == 'sub_fraction':
            if m['sub_frac'] < c[1]:
                errs.append(f"energy < 120 Hz only {m['sub_frac']:.2f} (< {c[1]})")
        elif c[0] == 'short':
            e1 = np.cumsum(m['mono'] ** 2)
            t90 = int(np.searchsorted(e1, 0.9 * e1[-1])) / SR
            if t90 > c[1]:
                errs.append(f'90% energy reached at {t90 * 1000:.0f} ms (> {c[1] * 1000:.0f} ms)')
        elif c[0] == 'ends_loudest':
            w = ns(0.1)
            r = env_rms(m['mono'], 100.0)
            end_lvl = rms(m['x'][-w:-ns(0.004)])
            if a2db(end_lvl) < a2db(r.max()) - 3.0:
                errs.append(f'end level {a2db(end_lvl):.1f} dB vs max {a2db(r.max()):.1f} dB')
        elif c[0] in ('rises', 'falls'):
            what, lim = c[1], c[2]
            part = c[3] if len(c) > 3 else None
            r = trend(m, what, part=part)
            m[f'trend_{what}'] = r
            if (c[0] == 'rises' and r < lim) or (c[0] == 'falls' and r > -lim):
                errs.append(f'{what} trend {r:+.2f}, expected {c[0]} (|r| >= {lim})')
        elif c[0] == 'no_presilence':
            lvl = a2db(rms(m['x'][:ns(0.03)]))
            if lvl < -60:
                errs.append(f'pre-silence: first 30 ms at {lvl:.1f} dBFS')
            lead = int(np.argmax(np.abs(m['x']).max(axis=1) > 0))
            if lead > ns(0.001):
                errs.append(f'{lead} leading zero samples (> 1 ms)')
    return errs


# ============================================================================ contact sheet


def contact_sheet(entries, metrics, path: Path, cols=6, dpi=56, title='ubiqX AI - synthesized SFX library'):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.gridspec import GridSpec

    bg, panel, ink, ink2, volt, ember, line = '#0a0d16', '#111726', '#e8edf9', '#a8b4cd', '#4d8dff', '#ff7a1f', '#1f2a40'
    rows = int(np.ceil(len(entries) / cols))
    fig = plt.figure(figsize=(cols * 5.2, rows * 3.5 + 0.8), facecolor=bg)
    fig.suptitle(title + f'  ({len(entries)} sounds, 48 kHz stereo 16-bit, peak -1 dBFS)', color=ink, fontsize=17,
                 y=0.998)
    outer = GridSpec(rows, cols, figure=fig, hspace=0.42, wspace=0.16, top=1 - 0.8 / (rows * 3.5 + 0.8),
                     bottom=0.01, left=0.02, right=0.99)
    for idx, e in enumerate(entries):
        m = metrics[e['file']]
        inner = outer[idx // cols, idx % cols].subgridspec(2, 1, height_ratios=[1, 1.9], hspace=0.04)
        ax1 = fig.add_subplot(inner[0])
        ax2 = fig.add_subplot(inner[1])
        x = m['x']
        n = x.shape[0]
        tt = np.arange(n) / SR
        bins = 700
        hop = max(1, n // bins)
        k = n // hop
        for c, col, al in ((0, volt, 0.9), (1, ember, 0.55)):
            seg = x[:k * hop, c].reshape(k, hop)
            ax1.fill_between(tt[:k * hop:hop], seg.min(axis=1), seg.max(axis=1), color=col, alpha=al, lw=0)
        ax1.set_xlim(0, n / SR)
        ax1.set_ylim(-1, 1)
        nper = int(2 ** np.clip(np.round(np.log2(n / 30)), 8, 11))
        f, ts, S = signal.spectrogram(m['mono'], SR, window='hann', nperseg=nper, noverlap=nper - nper // 8)
        S = 10 * np.log10(S + 1e-14)
        S = np.clip(S, S.max() - 70, None)
        fm = f > 0
        ax2.pcolormesh(ts, f[fm], S[fm], shading='auto', cmap='magma', rasterized=True)
        ax2.set_yscale('log')
        ax2.set_ylim(max(25.0, 1.5 * SR / nper), 22000)
        ax2.set_xlim(0, n / SR)
        for ax in (ax1, ax2):
            ax.set_facecolor(panel)
            ax.axvline(m['hit'], color='#2ecc8f', lw=1.1, alpha=0.9)
            for s in ax.spines.values():
                s.set_color(line)
            ax.tick_params(colors=ink2, labelsize=7, length=2)
        ax1.set_xticks([])
        ax1.set_yticks([])
        ax2.set_yticks([50, 200, 1000, 5000, 20000])
        ax2.set_yticklabels(['50', '200', '1k', '5k', '20k'])
        ax1.set_title(f"{e['file'][:-4]}  {m['dur']:.3f}s  pk {m['peak_dbfs']:.1f}  hit {m['hit'] * 1000:.0f}ms",
                      color=ink, fontsize=9.5, loc='left', pad=3)
    fig.savefig(str(path), dpi=dpi, facecolor=bg)
    plt.close(fig)


# ============================================================================ main


def build_manifest(entries, metrics):
    rows = []
    for e in entries:
        m = metrics[e['file']]
        rows.append(dict(
            name=e['name'], file=e['file'], variation=e['var'], duration_s=round(m['dur'], 4),
            peak_dbfs=round(m['peak_dbfs'], 2), hit_offset_s=round(m['hit'], 4),
            hit_offset_frames=round(m['hit'] * FPS, 2), end_is_downbeat=bool(e['eob']),
            suggested_use=e['use'], suggested_level_db=e['level'], rms_dbfs=round(m['rms_dbfs'], 1),
            character=e['character'], **({'pitch': e['pitch']} if e['pitch'] else {})))
    by_file = {r['file']: r for r in rows}
    for alias, target in ALIASES.items():
        if target in by_file:
            r = dict(by_file[target])
            r.update(name=alias[:-4], file=alias, alias_of=target,
                     suggested_use=f'Alias (byte copy) of {target} under the brief/style.md 7.5 name. ' + r['suggested_use'])
            rows.append(r)
    return rows


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--only', help='regex on file names to (re)build; others are only re-validated')
    ap.add_argument('--check', action='store_true', help='do not synthesize; validate files on disk')
    ap.add_argument('--no-sheet', action='store_true')
    ap.add_argument('--sheet', default=str(OUT / '_contact.png'))
    args = ap.parse_args(argv)
    OUT.mkdir(parents=True, exist_ok=True)

    todo = [e for e in REG if not args.check and (not args.only or re.search(args.only, e['file']))]
    for e in todo:
        key = f"{e['name']}_{e['var']}"
        x = e['fn'](key)
        if e['dur'][0] == 'exact' and x.shape[-1] != ns(e['dur'][1]):
            raise RuntimeError(f"{key}: {x.shape[-1]} samples, expected {ns(e['dur'][1])}")
        x = finalize(x, *e['fades'])
        write_wav(OUT / e['file'], x)
        print(f"  wrote {e['file']}", flush=True)

    metrics, bad = {}, 0
    present = [e for e in REG if (OUT / e['file']).exists()]
    print(f"\n{'file':24s} {'dur':>6s} {'peak':>6s} {'dBTP':>6s} {'rms':>6s} {'hit ms':>7s} {'<120Hz':>6s} "
          f"{'centr.':>6s} {'L/R r':>5s} {'tail':>6s}  status")
    for e in present:
        m = measure(e, OUT / e['file'])
        metrics[e['file']] = m
        errs = validate(e, m)
        bad += bool(errs)
        print(f"{e['file']:24s} {m['dur']:6.3f} {m['peak_dbfs']:6.2f} {m['tp_dbtp']:6.2f} {m['rms_dbfs']:6.1f} "
              f"{m['hit'] * 1000:7.1f} {m['sub_frac']:6.2f} {m['centroid']:6.0f} {m['lr']:5.2f} "
              f"{m.get('tail_db', float('nan')):6.1f}  {'OK' if not errs else 'FAIL: ' + '; '.join(errs)}")
    missing = [e['file'] for e in REG if e['file'] not in metrics]
    if missing:
        print('missing:', ', '.join(missing))
        bad += len(missing)

    for alias, target in ALIASES.items():
        if (OUT / target).exists():
            shutil.copyfile(OUT / target, OUT / alias)
    known = {e['file'] for e in REG} | set(ALIASES)
    stray = sorted(p.name for p in OUT.glob('*.wav') if p.name not in known)
    if stray:
        print('note: files not produced by this script:', ', '.join(stray))

    manifest = build_manifest(present, metrics)
    (OUT / 'sfx-manifest.json').write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + '\n')
    print(f"\nmanifest: {len(manifest)} rows ({len(present)} sounds + {len(manifest) - len(present)} aliases)")
    if not args.no_sheet:
        contact_sheet(present, metrics, Path(args.sheet))
        print('contact sheet:', args.sheet)
    print('VALIDATION', 'PASSED' if not bad else f'FAILED ({bad} files)')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
