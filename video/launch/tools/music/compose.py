#!/usr/bin/env python3
"""
ubiqX launch film — original music bed, synthesized from scratch.

    python3 tools/music/compose.py            # writes public/audio/music/{music.wav, stems/, markers.json}
    python3 tools/music/compose.py --fast     # skip the no-crash alt master

Deterministic: every random source is seeded from (instrument, sample position) with
crc32, so re-running reproduces the files bit-for-bit (numpy/scipy/numba versions held).

Spec: brief/storyboard.json "music" (A minor, 120 BPM 4/4, 1560 frames @ 30 fps = 52.000 s,
bar = 60 f, beat = 15 f). Every event is placed at an absolute film frame; one frame = 1600
samples at 48 kHz, so the file is exactly 2 496 000 samples.

Signal flow
    instruments (band-limited: polyBLEP saws, sines, FM, additive, filtered noise)
      -> per-stem dry + sends (hall / room convolution reverb, dotted-8th ping-pong delay)
      -> sidechain pump keyed to the kick -> stem bus (EQ, saturation, compression)
      -> global filter automation (under-water LP 0-224, HP lift 345-359, lift LP, build sweep,
         end-card LP close) -> stems (pre-master)
    sum of stems -> master: tilt EQ, glue compressor, soft saturation, gain to -14 LUFS,
      4x-oversampled true-peak limiter (-1.2 dBTP ceiling), TPDF dither -> music.wav

The intro (everything that starts before frame 225) is rendered and processed on its own
timeline and truncated at frame 225 with a 2 ms fade, so no reverb/delay/filter tail can
leak into the pre-reveal silence (225-239) or past it. Ducks for the SFX slams are NOT baked
in: src/shared/audio.ts applies them (MUSIC.ducking = true).
"""
from __future__ import annotations

import argparse
import json
import math
import os
import sys
import zlib

import numpy as np
from scipy import signal
import soundfile as sf
import pyloudnorm as pyln

try:
    from numba import njit
except Exception:  # pragma: no cover - pure-python fallback (slow, same maths)
    def njit(*a, **k):
        if a and callable(a[0]):
            return a[0]
        return lambda f: f

HERE = os.path.dirname(os.path.abspath(__file__))
PROJ = os.path.abspath(os.path.join(HERE, '..', '..'))
OUT_DIR = os.path.join(PROJ, 'public', 'audio', 'music')

SR = 48000
FPS = 30
SPF = SR // FPS            # 1600 samples per frame
NFR = 1560
N = NFR * SPF              # 2 496 000
BEAT = 15                  # frames
BAR = 60
STOP_F, DROP_F, FINAL_F = 225, 240, 1380
END_F = 1556               # tail fully faded by here (<= 1559)
TAU = 2 * math.pi


def S(frame: float) -> int:
    return int(round(frame * SPF))


def fbar(bar: int, beat: float = 1, six: float = 0) -> float:
    """Absolute frame of bar (1-based), beat (1-based), sixteenth offset."""
    return (bar - 1) * BAR + (beat - 1) * BEAT + six * BEAT / 4


NOTE_IDX = {'C': 0, 'D': 2, 'E': 4, 'F': 5, 'G': 7, 'A': 9, 'B': 11}


def midi(name: str) -> int:
    n = NOTE_IDX[name[0]]
    rest = name[1:]
    if rest and rest[0] in '#b':
        n += 1 if rest[0] == '#' else -1
        rest = rest[1:]
    return n + 12 * (int(rest) + 1)


def m2f(m: float) -> float:
    return 440.0 * 2 ** ((m - 69) / 12)


def hz(name: str) -> float:
    return m2f(midi(name))


def rng_for(tag: str, pos: int = 0) -> np.random.Generator:
    return np.random.default_rng(zlib.crc32(f'{tag}:{pos}'.encode()))


def db(x: float) -> float:
    return 10 ** (x / 20)


# ----------------------------------------------------------------------------------------
# DSP primitives
# ----------------------------------------------------------------------------------------

@njit(cache=False)
def _svf(x, fc, q, mode, sr):
    """Zavalishin TPT state-variable filter, per-sample cutoff. mode 0 LP, 1 BP, 2 HP."""
    n = x.shape[0]
    y = np.empty(n)
    ic1 = 0.0
    ic2 = 0.0
    k = 1.0 / q
    for i in range(n):
        f = fc[i]
        if f > 0.45 * sr:
            f = 0.45 * sr
        if f < 5.0:
            f = 5.0
        g = math.tan(math.pi * f / sr)
        a1 = 1.0 / (1.0 + g * (g + k))
        a2 = g * a1
        a3 = g * a2
        v3 = x[i] - ic2
        v1 = a1 * ic1 + a2 * v3
        v2 = ic2 + a2 * ic1 + a3 * v3
        ic1 = 2.0 * v1 - ic1
        ic2 = 2.0 * v2 - ic2
        if mode == 0:
            y[i] = v2
        elif mode == 1:
            y[i] = v1
        else:
            y[i] = x[i] - k * v1 - v2
    return y


def svf(x, fc, q=0.707, mode='lp'):
    x = np.asarray(x, dtype=np.float64)
    m = {'lp': 0, 'bp': 1, 'hp': 2}[mode]
    if x.ndim == 2:
        return np.stack([svf(c, fc, q, mode) for c in x])
    fcv = np.broadcast_to(np.asarray(fc, dtype=np.float64), x.shape).copy()
    return _svf(x, fcv, float(q), m, float(SR))


def butter_sos(order, f, kind):
    return signal.butter(order, f, btype=kind, fs=SR, output='sos')


def sosf(sos, x):
    return signal.sosfilt(sos, x, axis=-1)


def rbj(kind, f0, gain_db=0.0, q=0.707):
    """RBJ cookbook biquad as one SOS row."""
    A = 10 ** (gain_db / 40)
    w = TAU * f0 / SR
    cw, sw = math.cos(w), math.sin(w)
    alpha = sw / (2 * q)
    if kind == 'peak':
        b = [1 + alpha * A, -2 * cw, 1 - alpha * A]
        a = [1 + alpha / A, -2 * cw, 1 - alpha / A]
    elif kind == 'lowshelf':
        sa = 2 * math.sqrt(A) * alpha
        b = [A * ((A + 1) - (A - 1) * cw + sa), 2 * A * ((A - 1) - (A + 1) * cw), A * ((A + 1) - (A - 1) * cw - sa)]
        a = [(A + 1) + (A - 1) * cw + sa, -2 * ((A - 1) + (A + 1) * cw), (A + 1) + (A - 1) * cw - sa]
    elif kind == 'highshelf':
        sa = 2 * math.sqrt(A) * alpha
        b = [A * ((A + 1) + (A - 1) * cw + sa), -2 * A * ((A - 1) + (A + 1) * cw), A * ((A + 1) + (A - 1) * cw - sa)]
        a = [(A + 1) - (A - 1) * cw + sa, 2 * ((A - 1) - (A + 1) * cw), (A + 1) - (A - 1) * cw - sa]
    else:
        raise ValueError(kind)
    b = np.array(b) / a[0]
    a = np.array(a) / a[0]
    return np.concatenate([b, a])[None, :]


def eq(x, *bands):
    sos = np.concatenate([rbj(*b) for b in bands])
    return sosf(sos, x)


def pan2(x, p=0.0):
    """Equal-power pan of a mono signal, p in [-1, 1]."""
    a = (p + 1) * math.pi / 4
    return np.stack([x * math.cos(a), x * math.sin(a)]) * math.sqrt(2)


def to_stereo(x, p=0.0):
    return pan2(x, p) if x.ndim == 1 else x


def saw_blep(freq, n, phase0=0.0, sr=SR):
    """Band-limited sawtooth (polyBLEP). freq: scalar or per-sample array (Hz)."""
    dt = np.broadcast_to(np.asarray(freq, dtype=np.float64) / sr, (n,))
    ph = (phase0 + np.concatenate([[0.0], np.cumsum(dt[:-1])])) % 1.0
    y = 2.0 * ph - 1.0
    m1 = ph < dt
    t = ph[m1] / dt[m1]
    y[m1] -= t + t - t * t - 1.0
    m2 = ph > 1.0 - dt
    t = (ph[m2] - 1.0) / dt[m2]
    y[m2] -= t * t + t + t + 1.0
    return y


def saw_os(freq, n, phase0=0.0):
    """polyBLEP saw rendered at 2x and decimated (the residual BLEP aliasing folds above 20 kHz
    and is removed by the polyphase anti-alias filter)."""
    f = np.asarray(freq, dtype=np.float64)
    f2 = np.repeat(f, 2)[: 2 * n] if f.ndim else f
    return signal.resample_poly(saw_blep(f2, 2 * n, phase0, 2 * SR), 1, 2)[:n]


def sine_ph(freq, n, phase0=0.0):
    dt = np.broadcast_to(np.asarray(freq, dtype=np.float64) / SR, (n,))
    return TAU * (phase0 + np.concatenate([[0.0], np.cumsum(dt[:-1])]))


def adsr(n_gate, a, d, s, r, curve=4.0):
    """Exponential-ish ADSR; returns an envelope of length n_gate + r samples (times in s)."""
    na, nd, nr = max(1, int(a * SR)), max(1, int(d * SR)), max(1, int(r * SR))
    env = np.empty(n_gate + nr)
    t = np.arange(n_gate)
    att = 1 - np.exp(-curve * np.minimum(t, na) / na)
    att /= (1 - math.exp(-curve))
    dec = s + (1 - s) * np.exp(-curve * np.clip(t - na, 0, None) / nd)
    env[:n_gate] = np.where(t < na, att, dec)
    last = env[n_gate - 1] if n_gate > 0 else 0.0
    env[n_gate:] = last * np.exp(-6.9 * np.arange(nr) / nr)
    env[-min(64, nr):] *= np.linspace(1, 0, min(64, nr))
    return env


def fade_edges(x, n_in=24, n_out=240):
    x = x.copy()
    L = x.shape[-1]
    n_in, n_out = min(n_in, L), min(n_out, L)
    if n_in:
        x[..., :n_in] *= np.linspace(0, 1, n_in)
    if n_out:
        x[..., L - n_out:] *= np.linspace(1, 0, n_out)
    return x


def curve(points, n, kind='exp', start=0):
    """Per-sample automation from [(frame, value)], held flat outside. kind exp/lin."""
    fr = np.array([p[0] for p in points], dtype=np.float64) * SPF - start
    v = np.array([p[1] for p in points], dtype=np.float64)
    t = np.arange(n, dtype=np.float64)
    if kind == 'exp':
        return np.exp(np.interp(t, fr, np.log(v)))
    return np.interp(t, fr, v)


# ----------------------------------------------------------------------------------------
# Reverb / delay
# ----------------------------------------------------------------------------------------

def make_ir(rt_low, rt_mid, rt_high, length, predelay, seed, hp=160.0, lp=9000.0, er=True):
    n = int(length * SR)
    rng = np.random.default_rng(seed)
    t = np.arange(n) / SR
    out = np.zeros((2, n))
    lo_sos, hi_sos = butter_sos(2, 450, 'lowpass'), butter_sos(2, 4500, 'highpass')
    for ch in range(2):
        noise = rng.standard_normal(n)
        lo = sosf(lo_sos, noise)
        hi = sosf(hi_sos, noise)
        mid = noise - lo - hi
        e = lambda rt: 10 ** (-3 * t / rt)
        ir = lo * e(rt_low) + mid * e(rt_mid) + hi * e(rt_high)
        ir *= 1 - np.exp(-t / 0.012)      # density build-up
        if er:
            for k in range(7):
                d = int((0.006 + 0.0065 * k + rng.uniform(0, 0.004)) * SR)
                ir[d] += rng.choice([-1, 1]) * 0.9 * (0.72 ** k) * math.sqrt(np.mean(ir[:4800] ** 2)) * 25
        out[ch] = ir
    out = sosf(butter_sos(2, hp, 'highpass'), out)
    out = sosf(butter_sos(2, lp, 'lowpass'), out)
    out[:, -2400:] *= np.linspace(1, 0, 2400)
    pd = int(predelay * SR)
    out = np.concatenate([np.zeros((2, pd)), out], axis=1)
    out /= math.sqrt(np.sum(out ** 2) / 2)
    return out


IR_HALL = None
IR_ROOM = None


def irs():
    global IR_HALL, IR_ROOM
    if IR_HALL is None:
        IR_HALL = make_ir(1.9, 2.5, 1.1, 3.2, 0.024, 7001)
        IR_ROOM = make_ir(0.45, 0.6, 0.35, 0.9, 0.004, 7002, hp=250, lp=11000)
    return IR_HALL, IR_ROOM


def convolve(x, ir):
    y = np.stack([signal.oaconvolve(x[c], ir[c])[: x.shape[1]] for c in range(2)])
    return y


def pingpong(x, delay_s=0.375, fb=0.36, repeats=9):
    """Dotted-8th ping-pong: mono-summed input, repeats alternate L/R, each darker."""
    d = int(round(delay_s * SR))
    mono = 0.5 * (x[0] + x[1])
    out = np.zeros_like(x)
    tap = mono
    lp, hp = butter_sos(1, 3800, 'lowpass'), butter_sos(1, 320, 'highpass')
    for k in range(1, repeats + 1):
        tap = sosf(hp, sosf(lp, tap))
        g = fb ** (k - 1)
        ch = (k - 1) % 2
        if k * d >= x.shape[1]:
            break
        out[ch, k * d:] += g * tap[: x.shape[1] - k * d]
        out[1 - ch, k * d:] += 0.25 * g * tap[: x.shape[1] - k * d]
    return out


# ----------------------------------------------------------------------------------------
# Dynamics
# ----------------------------------------------------------------------------------------

@njit(cache=False)
def _comp_gain(det_db, thr, ratio, knee, a_att, a_rel):
    n = det_db.shape[0]
    g = np.empty(n)
    env = 0.0
    for i in range(n):
        over = det_db[i] - thr
        if 2 * over < -knee:
            gr = 0.0
        elif 2 * abs(over) <= knee:
            gr = (1.0 / ratio - 1.0) * (over + knee / 2) ** 2 / (2 * knee)
        else:
            gr = (1.0 / ratio - 1.0) * over
        if gr < env:
            env = a_att * env + (1 - a_att) * gr
        else:
            env = a_rel * env + (1 - a_rel) * gr
        g[i] = env
    return g


def compress(x, thr=-18.0, ratio=2.0, attack=0.01, release=0.15, knee=6.0, rms_ms=5.0, makeup=0.0):
    """Stereo-linked feed-forward compressor (RMS detector)."""
    p = np.mean(x ** 2, axis=0)
    w = max(1, int(rms_ms * 1e-3 * SR))
    p = np.convolve(p, np.ones(w) / w, mode='full')[: x.shape[1]]
    det = 10 * np.log10(p + 1e-12)
    g = _comp_gain(det, thr, ratio, knee, math.exp(-1 / (attack * SR)), math.exp(-1 / (release * SR)))
    return x * db(1) ** 0 * 10 ** ((g + makeup) / 20)


def saturate(x, drive=1.5, mix=1.0):
    y = np.tanh(drive * x) / math.tanh(drive)
    return mix * y + (1 - mix) * x


@njit(cache=False)
def _release_smooth(g, a_rel):
    n = g.shape[0]
    y = np.empty(n)
    e = 1.0
    for i in range(n):
        if g[i] < e:
            e = g[i]
        else:
            e = a_rel * e + (1 - a_rel) * g[i]
        y[i] = e
    return y


def true_peak(x):
    up = signal.resample_poly(x, 4, 1, axis=-1)
    return float(np.max(np.abs(up)))


def limiter(x, ceiling_db=-1.2, look_ms=1.5, release_ms=60.0):
    """Look-ahead true-peak limiter: peaks measured 4x oversampled, gain never lags a peak."""
    c = db(ceiling_db)
    up = signal.resample_poly(x, 4, 1, axis=-1)
    pk = np.max(np.abs(up), axis=0)
    pk = pk[: 4 * x.shape[1]].reshape(-1, 4).max(axis=1)
    req = np.minimum(1.0, c / np.maximum(pk, 1e-12))
    L = max(1, int(look_ms * 1e-3 * SR))
    # forward min over [n, n+L]
    padded = np.concatenate([req, np.ones(L)])
    from scipy.ndimage import minimum_filter1d
    mn = minimum_filter1d(padded, size=L + 1, origin=-(L // 2))[: len(req)]
    sm = _release_smooth(mn, math.exp(-1 / (release_ms * 1e-3 * SR)))
    # moving average over L (causal-ahead) keeps g <= req at every peak
    k = np.ones(L + 1) / (L + 1)
    g = np.convolve(np.concatenate([sm, np.full(L, sm[-1])]), k, mode='valid')[: len(req)]
    g = np.minimum(g, mn)
    return x * g, g


# ----------------------------------------------------------------------------------------
# Instruments (all return mono or (2, n) arrays at 48 kHz)
# ----------------------------------------------------------------------------------------

def kick(vel=1.0, kind='main', seed=0):
    n = int(0.55 * SR)
    t = np.arange(n) / SR
    if kind == 'main':
        f0, f1, tp, dec, click, drive = 175.0, 54.0, 0.030, 0.22, 0.55, 2.2
    elif kind == 'stab':
        f0, f1, tp, dec, click, drive = 175.0, 54.0, 0.030, 0.13, 0.6, 2.2
    elif kind == 'felt':
        f0, f1, tp, dec, click, drive = 115.0, 52.0, 0.042, 0.30, 0.0, 1.3
    elif kind == 'thump':
        f0, f1, tp, dec, click, drive = 90.0, 46.0, 0.05, 0.20, 0.0, 1.0
    else:  # 'final' — longer boom
        f0, f1, tp, dec, click, drive = 185.0, 52.0, 0.034, 0.55, 0.6, 2.4
    fr = f1 + (f0 - f1) * np.exp(-t / tp)
    ph = sine_ph(fr, n)
    amp = np.exp(-t / dec) * (1 - 0.35 * (1 - np.exp(-t / 0.08)))
    body = np.sin(ph) * amp
    body = np.tanh(drive * body) / math.tanh(drive)
    # knock: short 2nd layer around 110 Hz adds punch on small speakers
    knock = np.sin(sine_ph(110 + 90 * np.exp(-t / 0.01), n)) * np.exp(-t / 0.028) * 0.35
    y = body + (knock if kind in ('main', 'final') else 0)
    if click > 0:
        r = rng_for('kickclick', seed)
        nc = int(0.004 * SR)
        cl = r.standard_normal(nc) * np.exp(-np.arange(nc) / (0.0009 * SR))
        cl = sosf(butter_sos(2, [2500, 9000], 'bandpass'), cl)
        tick = np.sin(TAU * 3100 * t[:nc]) * np.exp(-t[:nc] / 0.0012)
        y[:nc] += click * (0.9 * cl / (np.max(np.abs(cl)) + 1e-9) + 0.35 * tick)
    y[-2400:] *= np.linspace(1, 0, 2400)
    return vel * y


def clap(vel=1.0, seed=0, tail=0.12, bright=1.0):
    n = int(0.5 * SR)
    t = np.arange(n) / SR
    r = rng_for('clap', seed)
    out = []
    for ch in range(2):
        noise = r.standard_normal(n)
        env = np.zeros(n)
        for ti, gi in zip((0.0, 0.0085, 0.0175, 0.027), (0.75, 0.7, 0.85, 1.0)):
            env += gi * np.where(t >= ti, np.exp(-(t - ti) / 0.0038), 0)
        env += 0.55 * np.where(t >= 0.027, np.exp(-(t - 0.027) / tail), 0)
        body = sosf(butter_sos(2, [850, 2300], 'bandpass'), noise)
        air = sosf(butter_sos(2, 6500, 'highpass'), noise) * 0.35 * bright
        out.append((body + air) * env)
    y = np.array(out)
    y /= np.max(np.abs(y)) + 1e-9
    return vel * fade_edges(y, 0, 2400)


def snare(vel=1.0, seed=0, tone=1.0, dec=0.14):
    n = int(0.5 * SR)
    t = np.arange(n) / SR
    r = rng_for('snare', seed)
    body = (np.sin(sine_ph(190 * (1 + 0.25 * np.exp(-t / 0.01)), n)) * np.exp(-t / 0.07)
            + 0.45 * np.sin(sine_ph(335, n)) * np.exp(-t / 0.045)) * tone
    out = []
    for ch in range(2):
        noise = r.standard_normal(n)
        nz = sosf(butter_sos(2, 1600, 'highpass'), noise)
        nz = eq(nz, ('peak', 4000, -5, 1.0), ('peak', 7500, 3, 0.8))
        out.append(0.7 * body + 0.55 * nz * np.exp(-t / dec))
    y = np.array(out)
    y /= np.max(np.abs(y)) + 1e-9
    return vel * fade_edges(y, 0, 2400)


HAT_PARTIALS = np.array([5130., 6015., 7340., 8150., 9310., 10480.])


def hat(vel=1.0, seed=0, open_=False, tone=0.0):
    dec = 0.30 if open_ else 0.030
    n = int((dec * 5 + 0.02) * SR)
    t = np.arange(n) / SR
    r = rng_for('hat', seed)
    noise = r.standard_normal(n)
    nz = sosf(butter_sos(4, 7200 + 600 * tone, 'highpass'), noise)
    metal = np.zeros(n)
    for f in HAT_PARTIALS * (1 + 0.02 * tone):
        metal += np.sin(TAU * f * t + r.uniform(0, TAU))
    metal = sosf(butter_sos(2, 6500, 'highpass'), metal) * 0.18
    env = np.exp(-t / dec) * (1 - np.exp(-t / 0.0003))
    if open_:
        env = 0.6 * env + 0.4 * np.exp(-t / 0.05)
    y = (nz + metal) * env
    y = sosf(butter_sos(2, 15500, 'lowpass'), y)
    y /= np.max(np.abs(y)) + 1e-9
    return vel * fade_edges(y, 0, 480)


def ride(vel=1.0, seed=0):
    n = int(1.2 * SR)
    t = np.arange(n) / SR
    r = rng_for('ride', seed)
    nz = sosf(butter_sos(3, 6000, 'highpass'), r.standard_normal(n)) * np.exp(-t / 0.35) * 0.6
    bell = np.zeros(n)
    for f, a, d in ((5260., 1.0, 0.5), (6330., 0.7, 0.4), (7910., 0.5, 0.3), (9670., 0.35, 0.25), (2630., 0.25, 0.3)):
        bell += a * np.sin(TAU * f * t + r.uniform(0, TAU)) * np.exp(-t / d)
    y = nz + 0.25 * bell
    y *= 1 - np.exp(-t / 0.0004)
    y /= np.max(np.abs(y)) + 1e-9
    return vel * fade_edges(pan2(y, 0.35), 0, 4800)


def crash(vel=1.0, seed=0, dur=2.6, tau=0.75):
    n = int(dur * SR)
    t = np.arange(n) / SR
    r = rng_for('crash', seed)
    out = []
    for ch in range(2):
        nz = r.standard_normal(n)
        nz = sosf(butter_sos(2, 2600, 'highpass'), nz)
        nz = sosf(butter_sos(2, 14500, 'lowpass'), nz)
        shimmer = np.zeros(n)
        for _ in range(18):
            f = r.uniform(3200, 11500)
            shimmer += np.sin(TAU * f * t + r.uniform(0, TAU)) * np.exp(-t / r.uniform(0.4, 1.4))
        env = 0.55 * np.exp(-t / tau) + 0.45 * np.exp(-t / 0.06)
        env *= 1 - np.exp(-t / 0.0006)
        y = nz * env + 0.05 * shimmer * np.exp(-t / tau)
        y = eq(y, ('peak', 4000, -5, 0.9), ('highshelf', 9000, 1.5, 0.7))
        out.append(y)
    y = np.array(out)
    y /= np.max(np.abs(y)) + 1e-9
    return vel * fade_edges(y, 0, int(0.3 * SR))


def tom(freq=110.0, vel=1.0, seed=0):
    n = int(0.6 * SR)
    t = np.arange(n) / SR
    r = rng_for('tom', seed)
    body = np.sin(sine_ph(freq * (1 + 0.5 * np.exp(-t / 0.025)), n)) * np.exp(-t / 0.28)
    stick = sosf(butter_sos(2, [900, 5000], 'bandpass'), r.standard_normal(n)) * np.exp(-t / 0.012) * 0.6
    y = np.tanh(1.4 * (body + stick))
    y /= np.max(np.abs(y)) + 1e-9
    return vel * fade_edges(y, 0, 2400)


def pluck(m, vel=1.0, seed=0, bright=1.0, decay=0.42, dur=None, width=0.25):
    """FM pluck (ratio 1 & 2, index envelope) + filtered polyBLEP saw, 2 detuned voices."""
    dur = dur or min(2.5, decay * 5 + 0.2)
    n = int(dur * SR)
    t = np.arange(n) / SR
    f = m2f(m)
    r = rng_for('pluck', seed)
    amp = (1 - np.exp(-t / 0.0012)) * np.exp(-t / decay)
    idx = 2.2 * bright * np.exp(-t / 0.05) + 0.35
    fc = f * 1.6 + 5200 * bright * np.exp(-t / 0.045)
    out = np.zeros((2, n))
    for cents, p in ((-5.0, -width), (5.0, width)):
        ff = f * 2 ** (cents / 1200)
        ph = r.uniform(0, 1)
        fm = np.sin(TAU * ff * t + idx * np.sin(TAU * ff * t + ph) + 0.3 * idx * np.sin(TAU * 2 * ff * t))
        saw = svf(saw_os(ff, n, ph), fc, 0.9, 'lp')
        v = (0.62 * fm + 0.45 * saw) * amp
        out += pan2(v, p) * 0.5
    return vel * fade_edges(out, 0, 1200)


BELL_PARTIALS = ((1.0, 1.0, 1.7), (2.0, 0.42, 0.95), (3.0, 0.16, 0.55), (4.16, 0.11, 0.38),
                 (5.43, 0.06, 0.26), (6.79, 0.035, 0.18))


def bell(m, vel=1.0, seed=0, decay=1.0, dur=None):
    dur = dur or 2.6 * decay + 0.3
    n = int(dur * SR)
    t = np.arange(n) / SR
    f = m2f(m)
    r = rng_for('bell', seed)
    out = np.zeros((2, n))
    for ch, cents in ((0, -1.5), (1, 1.5)):
        y = np.zeros(n)
        for ratio, a, d in BELL_PARTIALS:
            fr = f * ratio * 2 ** (cents / 1200)
            if fr > 16000:
                continue
            y += a * np.sin(TAU * fr * t + r.uniform(0, TAU)) * np.exp(-t / (d * decay))
        # soft FM shimmer on the fundamental
        y += 0.12 * np.sin(TAU * f * t + 0.8 * np.exp(-t / 0.08) * np.sin(TAU * 3.5 * f * t)) * np.exp(-t / (0.6 * decay))
        out[ch] = y * (1 - np.exp(-t / 0.0008))
    out /= np.max(np.abs(out)) + 1e-9
    return vel * fade_edges(out, 0, 2400)


def piano(m, vel=1.0, seed=0, dur=3.2):
    n = int(dur * SR)
    t = np.arange(n) / SR
    f0 = m2f(m)
    B = 0.00032
    r = rng_for('piano', seed)
    y = np.zeros(n)
    for k in range(1, 48):
        fk = k * f0 * math.sqrt(1 + B * k * k)
        if fk > 9000:
            break
        a = (1 / k ** 1.05) * (1.0 if k > 1 else 0.8) * math.exp(-k / 18)
        d = 3.2 / (1 + 0.16 * k)
        for dc in (-0.7, 0.0, 0.8):
            y += a / 3 * np.sin(TAU * fk * 2 ** (dc / 1200) * t + r.uniform(0, TAU)) * np.exp(-t / d)
    y *= 1 - np.exp(-t / 0.002)
    ham = sosf(butter_sos(2, 1100, 'lowpass'), r.standard_normal(n)) * np.exp(-t / 0.02) * 0.3
    y = y + ham
    y /= np.max(np.abs(y)) + 1e-9
    return vel * fade_edges(y, 0, 4800)


def supersaw(notes, gate_s, seed=0, voices=7, detune=16.0, width=0.85, cutoff=3000.0, q=0.75,
             a=0.02, d=0.3, s=0.8, r=0.35, env_cut=0.0, env_tau=0.12, lfo=0.12, lfo_hz=0.23, level=1.0):
    """Unison detuned polyBLEP saws per note, spread in stereo, SVF low-pass, ADSR."""
    n_gate = int(gate_s * SR)
    env = adsr(n_gate, a, d, s, r)
    n = len(env)
    t = np.arange(n) / SR
    rg = rng_for('ss', seed)
    spread = np.linspace(-1, 1, voices) if voices > 1 else np.zeros(1)
    L = np.zeros(2 * n)                 # oscillators run at 2x, decimated below
    R = np.zeros(2 * n)
    t2 = np.arange(2 * n) / (2 * SR)
    for m in notes:
        f = m2f(m)
        for v in range(voices):
            cents = detune * spread[v] + rg.uniform(-1.5, 1.5)
            drift = 1 + 0.0009 * np.sin(TAU * rg.uniform(0.1, 0.35) * t2 + rg.uniform(0, TAU))
            y = saw_blep(f * 2 ** (cents / 1200) * drift, 2 * n, rg.uniform(0, 1), 2 * SR)
            g = 1.0 if v == voices // 2 else 0.78
            ang = (spread[v] * width + 1) * math.pi / 4
            L += g * y * math.cos(ang)
            R += g * y * math.sin(ang)
    L = signal.resample_poly(L, 1, 2)[:n]
    R = signal.resample_poly(R, 1, 2)[:n]
    fc = cutoff * (1 + lfo * np.sin(TAU * lfo_hz * t + rg.uniform(0, TAU)))
    if env_cut:
        fc = fc + env_cut * np.exp(-t / env_tau)
    out = np.stack([svf(L, fc, q, 'lp'), svf(R, fc, q, 'lp')])
    out *= env / (math.sqrt(len(notes) * voices) * 1.4)
    return level * out


def bass_note(m, gate_s, vel=1.0, seed=0, cutoff=700.0, env_amt=1400.0, tau=0.07, sub=0.4):
    n_gate = int(gate_s * SR)
    env = adsr(n_gate, 0.002, 0.18, 0.55, 0.03)
    n = len(env)
    t = np.arange(n) / SR
    f = m2f(m)
    r = rng_for('bass', seed)
    y = 0.5 * saw_os(f * 2 ** (-6 / 1200), n, r.uniform(0, 1)) + 0.5 * saw_os(f * 2 ** (6 / 1200), n, r.uniform(0, 1))
    y = svf(y, cutoff + env_amt * np.exp(-t / tau), 1.1, 'lp')
    y += sub * np.sin(sine_ph(f / 2, n))
    y = np.tanh(1.6 * y * env) / math.tanh(1.6)
    return vel * y


def no_synth(vel=1.0, seed=0):
    """Low 'no' stab: A1 + A2 + E2 detuned saws, pitch falls a whole step, filter closes."""
    n_gate = int(0.34 * SR)
    env = adsr(n_gate, 0.003, 0.25, 0.6, 0.12)
    n = len(env)
    t = np.arange(n) / SR
    r = rng_for('no', seed)
    bend = 2 ** (-2 * np.clip((t - 0.1) / 0.24, 0, 1) ** 1.5 / 12)
    y = np.zeros((2, n))
    for m in (midi('A1'), midi('A2'), midi('E2')):
        for c, p in ((-9, -0.5), (0, 0.0), (9, 0.5)):
            v = saw_os(m2f(m) * 2 ** (c / 1200) * bend, n, r.uniform(0, 1))
            y += pan2(v, p) * 0.3
    fc = 180 + 2200 * np.exp(-t / 0.09)
    y = np.stack([svf(y[c], fc, 1.4, 'lp') for c in range(2)])
    y = np.tanh(2.0 * y * env) / math.tanh(2.0)
    y += pan2(np.sin(sine_ph(m2f(midi('A1')) * bend, n)) * env * 0.6)
    return vel * y


def noise_swell(dur_s, seed=0, f0=400.0, f1=6000.0, q=1.2):
    """Filtered noise swell rising into its end (for uplifters)."""
    n = int(dur_s * SR)
    t = np.linspace(0, 1, n)
    r = rng_for('swell', seed)
    nz = r.standard_normal((2, n))
    fc = f0 * (f1 / f0) ** (t ** 1.6)
    y = np.stack([svf(nz[c], fc, q, 'bp') for c in range(2)])
    y *= t ** 2.2
    y /= np.max(np.abs(y)) + 1e-9
    return y


def reverse(x, n_out):
    y = x[..., ::-1]
    return y[..., -n_out:] if y.shape[-1] >= n_out else y


# ----------------------------------------------------------------------------------------
# Mixer: stems x {dry, sends} on two timelines (intro < 225, main >= 240)
# ----------------------------------------------------------------------------------------

STEMS = ('drums', 'bass', 'harmony', 'lead', 'fx')
SENDS = ('hall', 'room', 'dly')
INTRO_N = S(STOP_F)


class Mixer:
    def __init__(self, exclude=()):
        self.exclude = set(exclude)
        self.buf = {seg: {} for seg in ('intro', 'main')}
        self.kicks = {'intro': [], 'main': []}
        self.log = []

    def seg_of(self, pos):
        if pos < INTRO_N:
            return 'intro'
        if pos < S(DROP_F):
            raise ValueError(f'event at sample {pos} falls in the 225-239 silence')
        return 'main'

    def get(self, seg, stem, bus):
        key = (stem, bus)
        if key not in self.buf[seg]:
            n = INTRO_N if seg == 'intro' else N
            self.buf[seg][key] = np.zeros((2, n))
        return self.buf[seg][key]

    def add(self, stem, frame, x, gain=1.0, pan=0.0, hall=0.0, room=0.0, dly=0.0, tag=None, offset=0):
        """Place signal x so that its sample 0 lands on `frame` (+offset samples)."""
        if tag and tag in self.exclude:
            return
        pos = S(frame) + int(offset)
        if pos < 0:
            x = x[..., -pos:]
            pos = 0
        seg = self.seg_of(max(pos, 0) if pos >= 0 else 0)
        x = to_stereo(np.asarray(x, dtype=np.float64), pan) * gain
        for bus, amt in (('dry', 1.0), ('hall', hall), ('room', room), ('dly', dly)):
            if amt <= 0:
                continue
            b = self.get(seg, stem, bus)
            end = min(b.shape[1], pos + x.shape[1])
            if end > pos:
                b[:, pos:end] += amt * x[:, : end - pos]

    def add_ending_at(self, stem, frame, x, **kw):
        """Place x so that its LAST sample is the sample before `frame` (reverse swells)."""
        pos = S(frame) - x.shape[-1]
        self.add(stem, 0, x, offset=pos, **kw)

    def kick_trig(self, frame, depth=1.0, offset=0):
        pos = S(frame) + offset
        self.kicks['intro' if pos < INTRO_N else 'main'].append((pos, depth))


def sidechain_env(kicks, n, depth, attack=0.003, release=0.19, shape=2.2):
    """Gain curve that dips at every kick (depth 0..1) and recovers with a smooth curve."""
    g = np.ones(n)
    na, nr = int(attack * SR), int(release * SR)
    ramp = np.concatenate([np.linspace(0, 1, na, endpoint=False), 1 - (np.linspace(0, 1, nr)) ** (1 / shape)])
    ramp = np.clip(ramp, 0, 1)
    for pos, dep in kicks:
        s0 = pos - na
        a0 = max(0, s0)
        a1 = min(n, s0 + len(ramp))
        if a1 <= a0:
            continue
        dip = 1 - depth * dep * ramp[a0 - s0: a1 - s0]
        g[a0:a1] = np.minimum(g[a0:a1], dip)
    # smooth the kink between overlapping recoveries
    return sosf(butter_sos(1, 120, 'lowpass'), g[None, :])[0]


SC_DEPTH = {'drums': 0.0, 'bass': 0.82, 'harmony': 0.5, 'lead': 0.18, 'fx': 0.12}


def stem_bus(stem, x, seg):
    if stem == 'drums':
        x = sosf(butter_sos(2, 28, 'highpass'), x)
        x = saturate(x, 1.4, 0.35)
        x = compress(x, thr=-14, ratio=2.5, attack=0.006, release=0.09, knee=6, rms_ms=3)
    elif stem == 'bass':
        x = sosf(butter_sos(2, 26, 'highpass'), x)
        x = saturate(x, 1.3, 0.5)
        x = sosf(butter_sos(2, 5200, 'lowpass'), x)
        x = compress(x, thr=-16, ratio=2.0, attack=0.015, release=0.12)
    elif stem == 'harmony':
        x = sosf(butter_sos(2, 95, 'highpass'), x)
        x = eq(x, ('peak', 320, -2.0, 0.9), ('peak', 3800, -1.5, 1.0), ('highshelf', 9000, 1.0, 0.7))
    elif stem == 'lead':
        x = sosf(butter_sos(2, 180, 'highpass'), x)
        x = eq(x, ('peak', 3600, -2.0, 1.1))
    elif stem == 'fx':
        x = sosf(butter_sos(2, 140, 'highpass'), x)
    return x


def automation(stem, x, seg):
    """Global filter moves (per stem, same curves -> the sum is the filtered mix)."""
    n = x.shape[1]
    if seg == 'intro':
        fc = curve([(0, 900), (180, 900), (224, 3000)], n)
        y = svf(svf(x, fc, 0.6, 'lp'), fc, 1.1, 'lp')          # 4-pole 'under water'
        return y * curve([(0, RIDE['intro']), (180, RIDE['intro']), (224, RIDE['intro_peak'])], n, 'lin')
    y = x
    # 345-359: quick high-pass lift into the match-cut, snaps back on 360
    hp = curve([(345, 22), (359.9, 1100)], n)
    wet = np.zeros(n)
    wet[S(345):S(360)] = 1
    wet = xfade_mask(wet, 96)
    y = y * (1 - wet) + svf(y, hp, 0.9, 'hp') * wet
    if stem in ('harmony', 'lead', 'fx'):
        # lift 1140-1259 slightly closed, breather warm, build sweeps open into 1380
        lp = curve([(1140, 5200), (1255, 4200), (1262, 2600), (1318, 2800), (1350, 1700), (1379.9, 17000)], n)
        w = np.zeros(n)
        w[S(1140):S(1380)] = 1
        w = xfade_mask(w, 480)
        y = y * (1 - w) + svf(y, lp, 0.8, 'lp') * w
    if stem != 'drums':
        hp2 = curve([(1350, 24), (1379.9, 320)], n)
        w = np.zeros(n)
        w[S(1350):S(1380)] = 1
        w = xfade_mask(w, 48)
        y = y * (1 - w) + svf(y, hp2, 0.8, 'hp') * w
    # end card: low-pass closes from 1500, level fades to zero by END_F
    lp3 = curve([(1500, 18000), (1535, 3200), (END_F, 220)], n)
    w = np.zeros(n)
    w[S(1500):] = 1
    y = y * (1 - w) + svf(y, lp3, 0.72, 'lp') * w
    y = y * ride_curve(n)
    fade = curve([(1500, 1.0), (1535, 0.5), (END_F - 1, 1e-3)], n)
    fade[S(END_F):] = 0
    y = y * fade
    return y


# Section fader rides (dB) — the energy map: drop loudest, features a notch under, lift and
# breather lower, the build climbs back into the final hit.
RIDE = {'intro': db(-5.0), 'intro_peak': db(-3.0)}
RIDE_POINTS = [(240, 0.0), (359.8, 0.0), (360, -1.5), (1139.8, -1.5), (1140, -2.5), (1259.8, -2.5), (1260, -4.0),
               (1320, -4.0), (1379.8, -1.0), (1380, 0.0)]


def ride_curve(n):
    return 10 ** (curve(RIDE_POINTS, n, 'lin') / 20)


def xfade_mask(w, nx):
    k = np.ones(nx) / nx
    return np.convolve(w, k, mode='same')


def render_stems(mx: Mixer):
    hall, room = irs()
    stems = {}
    for stem in STEMS:
        total = np.zeros((2, N))
        for seg in ('intro', 'main'):
            b = mx.buf[seg]
            n = INTRO_N if seg == 'intro' else N
            if not any(k[0] == stem for k in b):
                continue
            dry = b.get((stem, 'dry'), np.zeros((2, n)))
            wet = np.zeros((2, n))
            if (stem, 'dly') in b:
                d = pingpong(b[(stem, 'dly')])
                wet += d
                dry_hall = b.get((stem, 'hall'))
                b[(stem, 'hall')] = (dry_hall if dry_hall is not None else 0) + 0.35 * d
            if (stem, 'hall') in b:
                wet += convolve(b[(stem, 'hall')], hall)
            if (stem, 'room') in b:
                wet += convolve(b[(stem, 'room')], room)
            y = dry + wet
            if SC_DEPTH[stem] > 0 and mx.kicks[seg]:
                dep = SC_DEPTH[stem] * (0.6 if seg == 'intro' else 1.0)
                y = y * sidechain_env(mx.kicks[seg], n, dep)
            y = stem_bus(stem, y, seg)
            y = automation(stem, y, seg)
            if seg == 'intro':
                y[:, -96:] *= np.linspace(1, 0, 96)     # 2 ms hard stop at frame 225
                total[:, :n] += y
            else:
                y[:, : S(DROP_F)] = 0.0
                total += y
        total[:, S(STOP_F): S(DROP_F)] = 0.0
        total[:, S(END_F):] = 0.0
        stems[stem] = total
    return stems


# ----------------------------------------------------------------------------------------
# The score
# ----------------------------------------------------------------------------------------

EVENTS = []   # (frame, section, type, what) -> markers.json


def ev(frame, section, typ, what):
    EVENTS.append({'frame': round(frame, 3), 'section': section, 'type': typ, 'what': what})


V = midi  # shorthand
CH = {
    'Am9':   [V('A2'), V('E3'), V('B3'), V('C4'), V('G4')],
    'Am':    [V('E3'), V('A3'), V('C4'), V('E4')],
    'Amw':   [V('A2'), V('E3'), V('A3'), V('C4'), V('E4'), V('A4')],
    'F':     [V('F3'), V('A3'), V('C4'), V('E4')],
    'Fw':    [V('F2'), V('C3'), V('A3'), V('C4'), V('E4'), V('A4')],
    'C':     [V('E3'), V('G3'), V('C4'), V('E4')],
    'Cw':    [V('C3'), V('G3'), V('C4'), V('E4'), V('G4')],
    'G':     [V('D3'), V('G3'), V('B3'), V('D4')],
    'Gadd9': [V('G2'), V('D3'), V('A3'), V('B3'), V('D4')],
    'Fmaj9': [V('F2'), V('C3'), V('E3'), V('G3'), V('A3')],
    'Amadd9': [V('A2'), V('E3'), V('B3'), V('C4'), V('E4'), V('A4')],
    'Cadd9': [V('C3'), V('G3'), V('D4'), V('E4'), V('G4')],
}
ROOT = {'Am': 'A', 'Amw': 'A', 'Am9': 'A', 'Amadd9': 'A', 'F': 'F', 'Fw': 'F', 'Fmaj9': 'F',
        'C': 'C', 'Cw': 'C', 'Cadd9': 'C', 'G': 'G', 'Gadd9': 'G'}
SUB = {'A': V('A1'), 'F': V('F1'), 'C': V('C2'), 'G': V('G1')}
MIDB = {'A': V('A2'), 'F': V('F2'), 'C': V('C3'), 'G': V('G2')}

MOTIF_A = [(0, 'A4', 1.0), (3, 'C5', 0.78), (6, 'E5', 0.82), (8, 'G5', 0.9), (11, 'E5', 0.74), (14, 'C5', 0.7)]
MOTIF_B = [(0, 'A4', 1.0), (3, 'C5', 0.78), (6, 'E5', 0.82), (8, 'A5', 0.92), (11, 'G5', 0.76), (14, 'E5', 0.7)]
OVER_G = {'C5': 'B4', 'E5': 'D5', 'C6': 'B5'}

# Tonal SFX hits from storyboard.json scenes[].sfx (shimmer / bloop / chime / pop / key-down). The hook,
# arp and counter-melody leave [hit - 3.5, hit + 3] free so the SFX sound alone ("leave the frames where
# SFX hit free of melodic notes"). Designed unisons (420 ding, 885 shimmer, 900 bloop, 945 ding, 990
# click) are scripted stingers and stay.
TONAL_SFX = (250, 255, 675, 681, 708, 720, 813, 897, 993, 1203, 1320, 1390, 1395)


def sfx_clear(fr):
    return not any(h - 3.5 <= fr <= h + 3 for h in TONAL_SFX)


def motif_bar(mx, bar, chord, variant, gain, lo=0.0, hi=1e9, skip=(), bright=1.0, decay=0.42, dly=0.22, hall=0.18,
              holes=()):
    pat = MOTIF_A if variant == 0 else MOTIF_B
    for step, note, v in pat:
        fr = fbar(bar, 1, step)
        if fr < lo or fr >= hi or any(abs(fr - s) < 0.5 for s in skip) or any(a <= fr < b for a, b in holes) \
                or not sfx_clear(fr):
            continue
        nm = OVER_G.get(note, note) if ROOT[chord] == 'G' else note
        mx.add('lead', fr, pluck(V(nm), v, seed=S(fr), bright=bright, decay=decay), gain=gain,
               dly=dly, hall=hall)


def hats16(mx, f0, f1, gain, seed_tag='h', swing=0.085, open_every=None, skip=(), accent=(1.0, 0.55, 0.78, 0.6),
           room=0.12):
    """16th closed hats with humanized velocity and slight swing on the off-16ths."""
    fr = f0
    i = 0
    while fr < f1 - 1e-6:
        step = int(round((fr % BEAT) / (BEAT / 4))) % 4
        if not any(a <= fr < b for a, b in skip):
            r = rng_for(seed_tag, S(fr))
            vel = accent[step] * (1 + 0.14 * r.standard_normal())
            vel = float(np.clip(vel, 0.25, 1.2))
            off = (swing * 6000 if step % 2 == 1 else 0) + r.uniform(0, 90)   # late-only micro-timing
            is_open = open_every is not None and step == 2 and open_every(fr)
            h = hat(vel * (0.9 if is_open else 1.0), seed=S(fr), open_=is_open, tone=r.uniform(-0.5, 0.5))
            mx.add('drums', fr, h, gain=gain * (1.25 if is_open else 1.0), pan=0.22 if not is_open else -0.25,
                   room=room, offset=off)
        fr += BEAT / 4
        i += 1


def kick_at(mx, fr, vel=1.0, kind='main', sc=1.0, gain=1.0):
    mx.add('drums', fr, kick(vel, kind, seed=S(fr)), gain=gain)
    if sc:
        mx.kick_trig(fr, sc)


def clap_at(mx, fr, vel=1.0, gain=0.42, room=0.35, hall=0.08):
    mx.add('drums', fr, clap(vel, seed=S(fr)), gain=gain, room=room, hall=hall)


def crash_at(mx, fr, vel=1.0, gain=0.30, tag=None, dur=2.6, tau=0.75):
    mx.add('drums', fr, crash(vel, seed=S(fr), dur=dur, tau=tau), gain=gain, hall=0.12, tag=tag)


def pad(mx, f0, f1, chord, gain, stem='harmony', hall=0.3, dly=0.0, **kw):
    gate = (f1 - f0) / FPS
    mx.add(stem, f0, supersaw(CH[chord], gate, seed=S(f0) + len(chord), **kw), gain=gain, hall=hall, dly=dly)


def stab(mx, fr, chord, gain=0.55, bass=True, drums=True, clapv=1.0, sc=1.0, crash_=False, cutoff=5200):
    """Stop-time band hit: kick + clap + chord stab + bass stab together."""
    if drums:
        kick_at(mx, fr, 1.0, 'stab', sc)
        mx.add('drums', fr, clap(clapv, seed=S(fr), tail=0.07), gain=0.46, room=0.25, hall=0.06)
    mx.add('harmony', fr, supersaw(CH[chord] + [CH[chord][1] + 12], 0.16, seed=S(fr) + 3, voices=7, detune=20,
                                   width=0.95, cutoff=cutoff * 0.35, env_cut=cutoff, env_tau=0.07, a=0.001, d=0.12,
                                   s=0.45, r=0.12), gain=gain, hall=0.2, dly=0.05)
    if bass:
        mx.add('bass', fr, bass_note(MIDB[ROOT[chord]], 0.2, 1.0, seed=S(fr), cutoff=380, env_amt=1500), gain=0.5)
        nn = int(0.3 * SR)
        mx.add('bass', fr, np.sin(sine_ph(m2f(SUB[ROOT[chord]]), nn)) * np.exp(-np.arange(nn) / (0.09 * SR))
               * np.minimum(1, (nn - np.arange(nn)) / 480), gain=0.32)
    if crash_:
        crash_at(mx, fr, 1.0, gain=0.32)


def sub_line(mx, segs, gain=0.3, seg_gain=None):
    """Continuous sub sine per list of (f0, f1, root, level) with 5 ms ramps and phase continuity."""
    for f0, f1, root, lvl in segs:
        n = S(f1) - S(f0)
        t = np.arange(n) / SR
        f = m2f(SUB[root])
        y = np.sin(TAU * f * t) + 0.13 * np.sin(2 * TAU * f * t + 0.3)
        env = np.ones(n)
        k = min(240, n // 2)
        env[:k] = np.linspace(0, 1, k)
        env[-k:] = np.linspace(1, 0, k)
        mx.add('bass', f0, y * env * lvl, gain=gain)


def midbass_offbeats(mx, bar, root, gain=0.38, octave=False, skip=()):
    steps = (2, 6, 10, 14) if not octave else (0, 2, 4, 6, 8, 10, 12, 14)
    for st in steps:
        fr = fbar(bar, 1, st)
        if any(a <= fr < b for a, b in skip):
            continue
        m = MIDB[root] + (12 if octave and st % 4 == 2 else 0)
        if octave and st % 4 == 0:
            m = MIDB[root]
        v = 1.0 if st % 4 == 2 else 0.8
        mx.add('bass', fr, bass_note(m, 0.105, v, seed=S(fr), cutoff=520, env_amt=1300 if octave else 1000),
               gain=gain)


def build_score(mx: Mixer):
    EVENTS.clear()
    # ----------------------------- Bars 1-4 · Terça — under water (0-224) ------------------
    sec = 'Terça — under water'
    kick_at(mx, 0, 1.0, 'felt', 0.8, gain=1.3)
    mx.add('harmony', 0, supersaw(CH['Am'], 0.14, seed=11, voices=5, detune=12, cutoff=1400, env_cut=2600,
                                  env_tau=0.05, a=0.001, d=0.1, s=0.3, r=0.2), gain=0.95, hall=0.25)
    ev(0, sec, 'downbeat-hit', 'Frame-0 downbeat: felt kick + sub A1 + muted Am stab (A3-C4-E4), all through the 900 Hz low-pass. No lead-in: first sample is the hit.')
    pad(mx, 0, STOP_F + 2, 'Am9', 0.34, voices=5, detune=14, cutoff=1500, a=0.09, d=1.0, s=0.9, r=0.05, hall=0.4)
    sub_line(mx, [(0, STOP_F + 1, 'A', 1.0)], gain=0.26)
    for fr in (30, 60, 90, 120, 150, 180, 195, 210):
        kick_at(mx, fr, 0.95 if fr % 60 == 0 else 0.85, 'felt', 0.8, gain=1.05)
    for k in range(30):                                            # clock-like closed hats in 8ths
        fr = k * 7.5
        if fr >= STOP_F:
            break
        tick = k % 2 == 1
        mx.add('drums', fr, hat(0.9 if tick else 0.62, seed=S(fr), tone=0.6 if tick else -0.4), gain=0.2,
               pan=0.3 if tick else -0.1, room=0.05)
    # reversed-hat pickup that lands on 90
    rh = reverse(hat(1.0, seed=901, open_=True), int(0.32 * SR)) * np.linspace(0.2, 1, int(0.32 * SR)) ** 2
    mx.add_ending_at('fx', 90, rh, gain=0.34, hall=0.1)
    ev(90, sec, 'stinger', 'Reversed open-hat pickup swells and ends exactly on 90 (the cut to the chip montage).')
    # 105: muted bass stab
    mx.add('bass', 105, bass_note(V('A1'), 0.16, 1.0, seed=105, cutoff=300, env_amt=900, sub=0.6), gain=0.62)
    mx.add('harmony', 105, supersaw([V('A2'), V('E3')], 0.1, seed=1050, voices=5, cutoff=600, env_cut=1200,
                                    env_tau=0.04, a=0.001, d=0.08, s=0.2, r=0.12), gain=0.45)
    ev(105, sec, 'stinger', 'Muted bass stab A1/A2 on the “Só um minutinho.” slam (riser-2bar SFX starts here, not in the music).')
    # 150: low piano A1 + E2
    mx.add('harmony', 150, piano(V('A1'), 1.0, seed=1), gain=0.42, hall=0.3)
    mx.add('harmony', 150, piano(V('E2'), 0.8, seed=2), gain=0.36, hall=0.3)
    ev(150, sec, 'stinger', 'Low piano A1 + E2 (additive, inharmonic strings) as the counter lands on 40 min.')
    # foreshadow of the hook, deep under water (bar 3)
    motif_bar(mx, 3, 'Am', 0, gain=0.22, skip=(150,), hall=0.3, dly=0.25, bright=0.6)
    # snare build: 2 & 4 in bar 3, 8ths from 180, 16ths from 210
    for fr, v in ((135, 0.45), (165, 0.55)):
        mx.add('drums', fr, snare(v, seed=S(fr)), gain=0.4, room=0.3)
    roll = [(180 + 7.5 * i, 0.5 + 0.06 * i) for i in range(4)] + [(210 + 3.75 * i, 0.72 + 0.08 * i) for i in range(4)]
    for fr, v in roll:
        mx.add('drums', fr, snare(v, seed=S(fr), dec=0.1), gain=0.42, room=0.25)
    ev(180, sec, 'downbeat-hit', 'Bar 4 downbeat (“Chutar as horas?”): kick; snare 8ths 180-202.5, 16ths 210-221.25, crescendo.')
    ev(180, sec, 'filter-sweep', 'Under-water low-pass opens exponentially 900 Hz -> 3 kHz between 180 and 224 (Q 1.05, every stem).')
    ev(STOP_F, sec, 'stop', 'Hard stop of every stem at sample 360000 (2 ms fade); intro reverb/delay/filter tails are truncated with it.')
    ev(STOP_F, sec, 'silence', 'Frames 225-239 are digital zero in music.wav and every stem (no tail, no pickup, no dither).')

    # ----------------------------- Bars 5-6 · Drop (240-359) --------------------------------
    sec = 'Drop — ubiqX'
    kick_at(mx, 240, 1.0, 'final', 1.0, gain=1.05)
    crash_at(mx, 240, 1.0, gain=0.36, tag='crash240', dur=3.0, tau=0.9)
    pad(mx, 240, 300, 'Amw', 0.62, voices=7, detune=22, width=1.0, cutoff=6200, env_cut=5000, env_tau=0.12,
        a=0.001, d=0.5, s=0.72, r=0.12, hall=0.35)
    mx.add('harmony', 240, supersaw(CH['Amw'], 0.2, seed=2401, voices=7, detune=24, width=1.0, cutoff=1800,
                                    env_cut=9000, env_tau=0.05, a=0.001, d=0.12, s=0.3, r=0.25), gain=0.4, hall=0.3)
    ev(DROP_F, sec, 'drop', 'THE DROP: first sample of the main timeline. Long kick + sub A1 + crash + wide 7-voice supersaw Am (A2-A4), full range.')
    pad(mx, 300, 360, 'Fw', 0.55, voices=7, detune=22, width=1.0, cutoff=5200, env_cut=3000, a=0.002, d=0.5, s=0.7,
        r=0.1, hall=0.35)
    sub_line(mx, [(240, 300, 'A', 1.0), (300, 360, 'F', 1.0)])
    for b in (5, 6):
        midbass_offbeats(mx, b, 'A' if b == 5 else 'F', gain=0.36)
    for i in range(8):
        fr = 240 + 15 * i
        if fr > 240:
            kick_at(mx, fr)
    for fr in (255, 285, 315, 345):
        clap_at(mx, fr, 1.0)
    mx.add('drums', 255, snare(1.0, seed=255, dec=0.18), gain=0.36, room=0.4, hall=0.12)
    ev(255, sec, 'stinger', 'Snare accent layered on the beat-2 clap (UBI lands, descriptor finishes).')
    for i in range(8):
        fr = 247.5 + 15 * i
        mx.add('drums', fr, hat(1.0, seed=S(fr), open_=True), gain=0.2, pan=-0.2, room=0.15)
    hats16(mx, 240, 360, 0.19, skip=[(247, 248), (262, 263), (277, 278), (292, 293), (307, 308), (322, 323),
                                     (337, 338), (352, 353)])
    crash_at(mx, 300, 0.8, gain=0.26)
    motif_bar(mx, 6, 'Fw', 0, gain=0.5, bright=1.0)
    ev(300, sec, 'downbeat-hit', 'Bar 6: crash, chord to F (F2-A4 wide), the pluck hook A-C-E-G enters (FM + polyBLEP pluck, dotted-8th ping-pong).')
    rc = reverse(crash(1.0, seed=3591, dur=1.2, tau=0.5), int(0.5 * SR)) * np.linspace(0, 1, int(0.5 * SR)) ** 1.5
    mx.add_ending_at('fx', 360, rc, gain=0.2, hall=0.2)
    ev(345, sec, 'filter-sweep', 'High-pass lift on every stem, 22 Hz -> 1.1 kHz over 345-359, plus a reverse-crash swell ending on 360; snaps back to full range on 360.')

    # ----------------------------- Bars 7-11 · Groove A (360-659) ---------------------------
    sec = 'Groove A — registra e classifica'
    prog = {7: 'Am', 8: 'F', 9: 'C', 10: 'G', 11: 'Am'}
    for b, c in prog.items():
        f0, f1 = fbar(b), fbar(b + 1)
        if b == 9:
            pad(mx, f0, 510, c, 0.36, voices=7, detune=18, cutoff=4400, a=0.01, d=0.4, s=0.8, r=0.06, hall=0.35, dly=0.04)
        elif b == 11:
            pad(mx, f0, 645, c, 0.36, voices=7, detune=18, cutoff=4400, a=0.01, d=0.4, s=0.8, r=0.1, hall=0.35, dly=0.04)
        else:
            pad(mx, f0, f1, c, 0.36, voices=7, detune=18, cutoff=4400, a=0.01, d=0.4, s=0.8, r=0.08, hall=0.35, dly=0.04)
    sub_line(mx, [(360, 420, 'A', 1), (420, 480, 'F', 1), (480, 510, 'C', 1), (540, 600, 'G', 1), (600, 660, 'A', 1)])
    for b, c in prog.items():
        midbass_offbeats(mx, b, ROOT[c], gain=0.34, skip=[(510, 540), (645, 660)])
        motif_bar(mx, b, c, (b - 7) % 2, gain=0.44, skip=(420,), lo=0,
                  hi={11: 645, 9: 510}.get(b, 1e9))
    for i in range(20):
        fr = 360 + 15 * i
        if 510 <= fr < 541:
            continue
        kick_at(mx, fr)
    for fr in (375, 405, 435, 465, 495, 555, 615):
        clap_at(mx, fr, 1.0)
    clap_at(mx, 585, 0.9)
    hats16(mx, 360, 645, 0.19, skip=[(510, 540)], open_every=lambda fr: (fr // 60) in (6, 7, 9, 10) and int(fr) % 30 == 7)
    crash_at(mx, 360, 1.0, gain=0.3)
    ev(360, sec, 'downbeat-hit', 'Crash + groove A: 4-on-the-floor kick, clap 2/4, swung 16th hats, sub + off-beat mid bass on Am-F-C-G, pluck hook.')
    mx.add('lead', 420, pluck(V('C6'), 0.9, seed=4200, bright=1.1, decay=0.55), gain=0.34, dly=0.25, hall=0.2)
    ev(420, sec, 'stinger', 'Pluck C6 accent on the bar-8 downbeat (sits with ding_2); the hook note under it is kept, the rest of the bar is the normal hook.')
    mx.add('drums', 450, hat(1.0, seed=4500, open_=True), gain=0.2, pan=-0.3, room=0.2)
    sw = noise_swell(0.35, seed=450, f0=900, f1=7000) * 0.5
    mx.add_ending_at('fx', 450, sw, gain=0.12)
    ev(450, sec, 'stinger', 'Open-hat accent with a short band-passed noise swell ending on 450 (the whip into Categorias).')
    stab(mx, 510, 'C', gain=0.62)
    stab(mx, 525, 'C', gain=0.62)
    stab(mx, 540, 'G', gain=0.66, crash_=True)
    ev(510, sec, 'downbeat-hit', 'STOP-TIME stab 1 (beat 3, “Regras,”): kick + clap + C-chord supersaw stab + bass stab together; the groove is silent 510-539 between stabs.')
    ev(525, sec, 'downbeat-hit', 'Stop-time stab 2 (beat 4, “memória,”): same band hit on C.')
    ev(540, sec, 'downbeat-hit', 'Stop-time stab 3 + crash (bar 10 downbeat, “IA.”) on G; the groove resumes from here.')
    mx.add('drums', 585, tom(98, 1.0, seed=585), gain=0.5, room=0.3, pan=-0.15)
    ev(585, sec, 'stinger', 'Floor-tom accent (G2-ish, pitch drop) with the beat-4 clap on the cut to Revisão.')
    fill = [(645, tom(146, 0.8, 1), -0.3), (648.75, snare(0.75, 6487), 0.0), (652.5, tom(123, 0.9, 2), 0.1),
            (656.25, snare(1.0, 6562), 0.0)]
    for fr, x, p in fill:
        mx.add('drums', fr, x, gain=0.46, pan=p, room=0.35)
    ev(645, sec, 'filter-sweep', 'One-beat fill 645-659: tom-snare-tom-snare 16ths, crescendo; hook, pad and hats stop at 645 to set up the stop.')

    # ----------------------------- Bars 12-15 · Groove B (660-899) --------------------------
    sec = 'Groove B — revisão, com stop-time'
    stab(mx, 660, 'Am', gain=0.66)
    n_tail = S(690) - S(660)
    tt = np.arange(n_tail) / SR
    mx.add('bass', 660, np.sin(TAU * m2f(SUB['A']) * tt) * np.exp(-tt / 0.5) * np.minimum(1, (n_tail - np.arange(n_tail)) / 480),
           gain=0.26)
    for i, fr in enumerate((660, 667.5, 675, 682.5)):
        mx.add('drums', fr, hat(0.35 if fr == 675 else 0.85, seed=S(fr)), gain=0.17, pan=0.25, room=0.05)
    ev(660, sec, 'stop', 'Band STOP: one Am stab on the downbeat, then only the sub tail (A1, 0.5 s decay) and closed-hat 8ths; the 675 hat is -8 dB and nothing melodic sounds on 675 (key-down alone).')
    chords_b = [(690, 720, 'Am'), (720, 780, 'F'), (780, 810, 'Am'), (810, 855, 'C'), (855, 900, 'G')]
    for f0, f1, c in chords_b:
        bright = 6000 if f0 >= 810 else 4400
        pad(mx, f0, f1, c, 0.38 if f0 >= 810 else 0.35, voices=7, detune=18, cutoff=bright, a=0.01 if f0 != 690 else 0.002,
            d=0.4, s=0.8, r=0.08, hall=0.35, dly=0.04)
    sub_line(mx, [(690, 720, 'A', 1), (720, 780, 'F', 1), (780, 810, 'A', 1), (810, 855, 'C', 1), (855, 900, 'G', 1)])
    for b in (12, 13, 14, 15):
        root = {12: 'A', 13: 'F', 14: 'A', 15: 'C'}[b]
        for st in (2, 6, 10, 14):
            fr = fbar(b, 1, st)
            if fr < 690:
                continue
            rr = root
            if b == 14 and fr >= 810:
                rr = 'C'
            if b == 15 and fr >= 855:
                rr = 'G'
            mx.add('bass', fr, bass_note(MIDB[rr], 0.105, 1.0, seed=S(fr), cutoff=520, env_amt=1000), gain=0.34)
    for b in (12, 13, 14, 15):
        cm = {12: 'Am', 13: 'F', 14: 'Am', 15: 'C'}[b]
        motif_bar(mx, b, cm, (b - 12) % 2, gain=0.34, lo=690, skip=(810, 885, 900))
    for i in range(14):
        fr = 690 + 15 * i
        kick_at(mx, fr)
    for fr in (705, 735, 765, 795, 825, 855, 885):
        clap_at(mx, fr, 1.0)
    hats16(mx, 690, 900, 0.19, open_every=lambda fr: int(fr) % 30 == 7)
    counter = [(690, 'E5', 0.9), (697.5, 'G5', 0.8), (712.5, 'A5', 1.0), (727.5, 'G5', 0.8), (735, 'E5', 0.8),
               (750, 'A5', 0.9), (780, 'E5', 0.85), (787.5, 'G5', 0.8), (795, 'A5', 0.95), (840, 'A5', 0.9),
               (855, 'G5', 0.8), (862.5, 'E5', 0.8)]
    for fr, nm, v in counter:
        if not sfx_clear(fr):
            continue
        mx.add('lead', fr, bell(V(nm), v, seed=S(fr), decay=0.8), gain=0.2, hall=0.3, dly=0.2)
    ev(690, sec, 'downbeat-hit', 'Groove re-enters on beat 3 (key -> chip morph): kick, clap, hats, sub, bass, hook + bell counter-melody E5-G5-A5 (C-major pentatonic, additive bell).')
    crash_at(mx, 720, 0.6, gain=0.18, dur=1.8, tau=0.5)
    ev(720, sec, 'downbeat-hit', 'Bar 13 downbeat (rule chips): soft crash, chord to F.')
    mx.add('drums', 780, hat(1.0, seed=7800, open_=True), gain=0.22, pan=-0.2, room=0.2)
    ev(780, sec, 'downbeat-hit', 'Bar 14 downbeat (cut to “Confirmar os 19”): open-hat accent; a reverse crash swell starts here and ends on 810.')
    rc = reverse(crash(1.0, seed=8101, dur=1.4, tau=0.6), int(0.95 * SR)) * np.linspace(0, 1, int(0.95 * SR)) ** 1.8
    mx.add_ending_at('fx', 810, rc, gain=0.24, hall=0.2)
    stab(mx, 810, 'Cw', gain=0.7, crash_=True, bass=False)
    ev(810, sec, 'stinger', 'THE CLICK: full-band stab + crash, harmony lifts Am -> C with the pad filter opening (3.6 -> 5.2 kHz) — peak of the features act.')
    sparkle = ['C6', 'D6', 'E6', 'G6', 'A6', 'C7']
    for i, nm in enumerate(sparkle):
        fr = 885 + i * 2.5
        mx.add('lead', fr, bell(V(nm), 0.55 + 0.07 * i, seed=S(fr), decay=0.5), gain=0.13, hall=0.35, dly=0.25,
               pan=-0.5 + 0.2 * i)
    ev(885, sec, 'stinger', 'C-major-pentatonic sparkle C6-D6-E6-G6-A6-C7 in 12ths of a beat, 885-897.5 (sits with shimmer_2).')

    # ----------------------------- Bars 16-19 · Groove C (900-1139) -------------------------
    sec = 'Groove C — entrega e foco'
    prog_c = {16: 'Am', 17: 'F', 18: 'C', 19: 'G'}
    for b, c in prog_c.items():
        f0, f1 = fbar(b), fbar(b + 1)
        if b == 18:
            pad(mx, 1050, f1, c, 0.36, voices=7, detune=18, cutoff=4400, a=0.004, d=0.4, s=0.8, r=0.08, hall=0.35, dly=0.04)
            pad(mx, f0, 1035, c, 0.36, voices=7, detune=18, cutoff=4400, a=0.01, d=0.4, s=0.8, r=0.04, hall=0.35, dly=0.04)
        else:
            pad(mx, f0, f1, c, 0.36, voices=7, detune=18, cutoff=4400, a=0.01, d=0.4, s=0.8, r=0.08, hall=0.35, dly=0.04)
    sub_line(mx, [(900, 960, 'A', 1), (960, 1020, 'F', 1), (1020, 1035, 'C', 1), (1050, 1080, 'C', 1), (1080, 1140, 'G', 1)])
    for b, c in prog_c.items():
        midbass_offbeats(mx, b, ROOT[c], gain=0.3, octave=True, skip=[(1035, 1050)])
        motif_bar(mx, b, c, (b - 16) % 2, gain=0.36, skip=(900, 945, 990), bright=1.15, holes=[(1035, 1050)])
        arp = {'Am': ['A5', 'C6', 'E6', 'C6'], 'F': ['F5', 'A5', 'C6', 'A5'], 'C': ['G5', 'C6', 'E6', 'C6'],
               'G': ['G5', 'B5', 'D6', 'B5']}[c]
        for st in range(16):
            fr = fbar(b, 1, st)
            if 1035 <= fr < 1050 or st % 4 == 0 or not sfx_clear(fr):
                continue
            r = rng_for('arp', S(fr))
            mx.add('lead', fr, pluck(V(arp[st % 4]), 0.5 * (1 + 0.1 * r.standard_normal()), seed=S(fr) + 1,
                                     bright=0.8, decay=0.12, dur=0.5, width=0.5), gain=0.12, pan=0.35 if st % 2 else -0.35,
                   dly=0.12, hall=0.1)
    for i in range(16):
        fr = 900 + 15 * i
        if 930 <= fr < 960 or fr == 1035:
            continue
        kick_at(mx, fr, 0.8 if fr == 960 else 1.0)
    clap_at(mx, 900, 0.9, gain=0.36)
    for fr in (915, 975, 1005, 1065, 1095, 1125):
        clap_at(mx, fr, 1.0)
    for i in range(16):
        fr = 900 + 15 * i
        if 930 <= fr < 960 or fr == 1035:
            continue
        mx.add('drums', fr, ride(0.8, seed=S(fr)), gain=0.13, room=0.1)
    hats16(mx, 900, 1140, 0.16, skip=[(930, 960), (1035, 1050)])
    for fr in (930, 937.5, 945, 952.5):
        mx.add('drums', fr, hat(0.7, seed=S(fr) + 7), gain=0.15, pan=0.25)
    mx.add('lead', 900, bell(V('A5'), 1.0, seed=9000, decay=1.1), gain=0.24, hall=0.35, dly=0.2)
    ev(900, sec, 'downbeat-hit', 'UBI lands: clap on the downbeat + A5 bell; groove C = same drums + ride quarters, octave 8th bass, bright 16th arp over the hook.')
    ev(930, sec, 'stop', 'Thinned drums 930-959: kick, clap, ride and 16ths out; only soft closed-hat 8ths under the clock.')
    mx.add('lead', 945, bell(V('E6'), 0.9, seed=9450, decay=0.9), gain=0.2, hall=0.35, dly=0.2)
    ev(945, sec, 'stinger', 'Bell E6 as the clock lands on 18:00 (sits with ding_1).')
    crash_at(mx, 960, 0.5, gain=0.16, dur=1.8, tau=0.45)
    ev(960, sec, 'downbeat-hit', 'Soft hit (kick 0.8 + soft crash): drums back in as the report window rises.')
    mx.add('lead', 990, pluck(V('C6'), 0.9, seed=9900, bright=1.1, decay=0.5), gain=0.3, dly=0.25, hall=0.2)
    ev(990, sec, 'stinger', 'Pluck C6 with the “Copiar Markdown” click.')
    stab(mx, 1035, 'Am', gain=0.66)
    mx.add('bass', 1035, no_synth(1.0, seed=1035), gain=0.62, hall=0.08)
    ev(1035, sec, 'stinger', 'Full-band Am stab + low “no” synth (A1/E2/A2 saws, pitch falls a whole step, filter closes); groove holes 1035-1049 and resumes on 1050.')
    crash_at(mx, 1080, 0.7, gain=0.2, dur=2.0, tau=0.55)
    ev(1080, sec, 'downbeat-hit', 'Bar 19 downbeat (“Ok, foco!”): light crash, chord to G.')
    mx.add('drums', 1125, hat(1.0, seed=11250, open_=True), gain=0.22, pan=-0.25, room=0.2)
    mx.add('drums', 1122.5, snare(0.5, seed=11225, dec=0.08), gain=0.3, room=0.3)
    ev(1125, sec, 'stinger', 'Snare flam + open hat on beat 4 (the whip into Configurações).')

    # ----------------------------- Bars 20-21 · Lift (1140-1259) ----------------------------
    sec = 'Lift — a sua IA'
    pad(mx, 1140, 1200, 'Am9', 0.42, voices=7, detune=12, cutoff=1500, a=0.25, d=1.0, s=0.9, r=0.1, hall=0.5, q=0.6)
    pad(mx, 1200, 1262, 'Gadd9', 0.42, voices=7, detune=12, cutoff=1500, a=0.08, d=1.0, s=0.9, r=0.12, hall=0.5, q=0.6)
    sub_line(mx, [(1140, 1200, 'A', 0.9), (1200, 1260, 'G', 0.9)])
    for b, c in ((20, 'Am'), (21, 'G')):
        motif_bar(mx, b, c, (b - 20) % 2, gain=0.3, bright=0.8, dly=0.3, hall=0.3)
    for i in range(6):
        fr = 1140 + 15 * i
        kick_at(mx, fr, 0.9)
    hats16(mx, 1140, 1230, 0.11)
    ev(1140, sec, 'downbeat-hit', 'Lift: drums thin to kick + 16th hats (no clap), warm Am9 pad (1.5 kHz, slow attack) enters, hook/pads under a 5.2 kHz low-pass.')
    ev(1200, sec, 'downbeat-hit', 'Bar 21 downbeat (“IA do Ubi” click): pad moves to G(add9).')
    ev(1230, sec, 'stop', 'Drums out on beat 3 (last kick 1215, hats end 1229); pad, sub and hook carry into the breather.')

    # ----------------------------- Bars 22-23 · Respiro + build (1260-1379) -----------------
    sec = 'Respiro — privacidade, then the build'
    pad(mx, 1260, 1290, 'Fmaj9', 0.44, voices=7, detune=12, cutoff=1800, a=0.06, d=1.0, s=0.9, r=0.25, hall=0.55, q=0.6)
    pad(mx, 1290, 1320, 'Am9', 0.44, voices=7, detune=12, cutoff=1800, a=0.12, d=1.0, s=0.9, r=0.2, hall=0.55, q=0.6)
    pad(mx, 1320, 1380, 'Gadd9', 0.42, voices=7, detune=14, cutoff=2200, a=0.1, d=1.0, s=0.95, r=0.02, hall=0.45,
        env_cut=0, lfo=0.05)
    for fr, root in ((1260, 'F'), (1275, 'F'), (1290, 'A'), (1305, 'A')):
        n = int(0.42 * SR)
        tt = np.arange(n) / SR
        y = np.sin(TAU * m2f(SUB[root]) * tt) * (1 - np.exp(-tt / 0.008)) * np.exp(-tt / 0.18)
        y[-480:] *= np.linspace(1, 0, 480)
        mx.add('bass', fr, y, gain=0.36)
    ev(1260, sec, 'downbeat-hit', 'Breather: no kick; Fmaj9 pad (F2-C3-E3-G3-A3), sub pulse on quarters (F1).')
    mx.add('lead', 1275, pluck(V('E5'), 0.7, seed=12750, bright=0.6, decay=0.6), gain=0.3, dly=0.35, hall=0.4)
    mx.add('lead', 1297.5, pluck(V('C5'), 0.55, seed=12975, bright=0.6, decay=0.6), gain=0.26, dly=0.3, hall=0.4)
    ev(1275, sec, 'stinger', 'Soft pluck E5 on beat 2 (address chip); pad moves to Am9 on 1290 with A1 sub pulses.')
    sub_line(mx, [(1320, 1380, 'G', 0.85)])
    for i in range(4):
        fr = 1320 + 15 * i
        kick_at(mx, fr, 0.8 + 0.05 * i)
    ev(1320, sec, 'downbeat-hit', 'Kick returns on quarters (1320-1365) under riser_1bar_1; pad to G(add9), sub G1.')
    # rising 16th hook arpeggio through the build
    seq = ['A4', 'C5', 'E5', 'G5']
    for st in range(4, 16):
        fr = fbar(23, 1, st)
        nm = seq[st % 4]
        mx.add('lead', fr, pluck(V(nm) + (12 if st >= 12 else 0), 0.35 + 0.05 * (st - 4), seed=S(fr) + 5, bright=0.7 + 0.04 * st,
                                 decay=0.15, dur=0.5), gain=0.24, dly=0.1, hall=0.15)
    roll = [(1350, 0.55), (1357.5, 0.62), (1365, 0.7), (1368.75, 0.78), (1372.5, 0.88), (1376.25, 1.0)]
    for fr, v in roll:
        mx.add('drums', fr, snare(v, seed=S(fr), dec=0.1), gain=0.44, room=0.3)
    ev(1350, sec, 'filter-sweep', 'Snare roll 8ths (1350, 1357.5) -> 16ths (1365-1376.25); HPF 24 -> 320 Hz and LPF 1.7 -> 17 kHz sweep up on every non-drum stem into 1380.')

    # ----------------------------- Bars 24-26 · Final (1380-1559) ---------------------------
    sec = 'Final — end card'
    kick_at(mx, 1380, 1.0, 'final', 0.6, gain=1.08)
    crash_at(mx, 1380, 1.0, gain=0.34, dur=3.6, tau=1.0)
    mx.add('harmony', 1380, supersaw(CH['Amadd9'], 0.25, seed=13801, voices=7, detune=24, width=1.0, cutoff=2000,
                                     env_cut=9000, env_tau=0.06, a=0.001, d=0.15, s=0.3, r=0.3), gain=0.42, hall=0.4)
    pad(mx, 1380, 1412, 'Amadd9', 0.58, voices=7, detune=22, width=1.0, cutoff=5200, env_cut=4000, a=0.001, d=0.6,
        s=0.75, r=0.35, hall=0.5)
    pad(mx, 1410, 1522, 'Cadd9', 0.5, voices=7, detune=20, width=1.0, cutoff=3600, a=0.12, d=1.5, s=0.8, r=0.8,
        hall=0.6, lfo=0.06)
    n = S(1525) - S(1380)
    tt = np.arange(n) / SR
    fsub = np.where(tt < 1.0, m2f(SUB['A']), m2f(SUB['C']))
    ysub = np.sin(sine_ph(sosf(butter_sos(1, 8, 'lowpass'), fsub - fsub[0]) + fsub[0], n)) * np.exp(-tt / 1.6)
    ysub[-2400:] *= np.linspace(1, 0, 2400)
    mx.add('bass', 1380, ysub, gain=0.38)
    ev(FINAL_F, sec, 'downbeat-hit', 'FINAL HIT: long kick + sub A1 + crash + wide Am(add9) supersaw; drums end here.')
    mx.add('lead', 1380, pluck(V('A4'), 0.8, seed=13800, bright=0.8, decay=0.6), gain=0.3, dly=0.45, hall=0.4)
    mx.add('lead', 1410, pluck(V('E5'), 0.6, seed=14100, bright=0.7, decay=0.6), gain=0.24, dly=0.45, hall=0.4)
    ev(1410, sec, 'chord', 'Am(add9) resolves to Cadd9 (C3-G3-D4-E4-G4), sub glides A1 -> C2; slow pluck echo A4 (1380), E5 (1410) on long ping-pong delay.')
    kick_at(mx, 1395, 0.7, 'thump', 0, gain=0.8)
    ev(1395, sec, 'stinger', 'Soft low thump (90 -> 46 Hz sine, no click) as UBI lands on the lockup (sits with bloop_1).')
    mx.add('lead', 1440, pluck(V('C5'), 0.8, seed=14400, bright=0.7, decay=0.7), gain=0.3, dly=0.3, hall=0.45)
    mx.add('lead', 1440, bell(V('C5'), 0.6, seed=14401, decay=0.9), gain=0.12, hall=0.4)
    mx.add('lead', 1455, pluck(V('A4'), 0.75, seed=14550, bright=0.6, decay=0.8), gain=0.3, dly=0.3, hall=0.45)
    mx.add('lead', 1455, bell(V('A4'), 0.55, seed=14551, decay=0.9), gain=0.12, hall=0.4)
    ev(1440, sec, 'stinger', 'Two-note goodbye: C5 (1440) -> A4 (1455), pluck + bell, as UBI waves.')
    ev(1500, sec, 'filter-sweep', 'Tail: low-pass closes 18 kHz -> 180 Hz on every stem while the level fades; reverb decays inside the fade.')
    ev(END_F, sec, 'stop', f'Last non-zero sample is before frame {END_F} (sample {S(END_F)}); frames {END_F}-1559 are digital zero.')


# ----------------------------------------------------------------------------------------
# Master
# ----------------------------------------------------------------------------------------

STEM_GAIN = {'drums': 1.0, 'bass': db(1.5), 'harmony': db(3.5), 'lead': db(1.0), 'fx': db(2.0)}


def premaster(stems):
    return sum(STEM_GAIN[k] * v for k, v in stems.items())


def master_chain(mix, gain_db=None, target=-14.0, ceiling=-1.2):
    x = sosf(butter_sos(2, 22, 'highpass'), mix)
    x = eq(x, ('lowshelf', 48, -1.5, 0.7), ('peak', 3800, -1.0, 0.9), ('highshelf', 9000, 2.5, 0.7))
    x = compress(x, thr=-12, ratio=1.5, attack=0.015, release=0.2, knee=8, rms_ms=12)
    meter = pyln.Meter(SR)
    if gain_db is None:
        g = 0.0
        for _ in range(6):
            y = saturate(x * db(g), 1.15, 0.5)
            y, _gr = limiter(y, ceiling)
            lufs = meter.integrated_loudness(y.T)
            if abs(lufs - target) < 0.03:
                break
            g += target - lufs
        gain_db = g
    y = saturate(x * db(gain_db), 1.15, 0.5)
    y, gr = limiter(y, ceiling)
    return y, gain_db, gr


def finalize(y):
    """TPDF dither to 16-bit, then force the silence regions to digital zero."""
    r = np.random.default_rng(160)
    q = 1 / 32768
    d = (r.uniform(-0.5, 0.5, y.shape) + r.uniform(-0.5, 0.5, y.shape)) * q
    z = np.clip(np.round((y + d) * 32767), -32767, 32767).astype(np.int16)
    z[:, S(STOP_F): S(DROP_F)] = 0
    z[:, S(END_F):] = 0
    # no dither on digitally-silent input
    z[:, np.all(np.abs(y) < 1e-9, axis=0)] = 0
    return z


def write_wav(path, x, subtype='PCM_16'):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if x.dtype == np.int16:
        sf.write(path, x.T, SR, subtype='PCM_16')
    else:
        sf.write(path, np.clip(x, -1, 1).T, SR, subtype=subtype)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--fast', action='store_true', help='skip the no-crash alternate master')
    ap.add_argument('--out', default=OUT_DIR)
    args = ap.parse_args()

    mx = Mixer()
    build_score(mx)
    stems = render_stems(mx)
    mix = premaster(stems)
    pk = float(np.max(np.abs(mix)))
    stem_scale = 0.89 / pk if pk > 0.89 else 1.0      # pre-master stems keep headroom
    master, gain_db, gr = master_chain(mix)
    out16 = finalize(master)
    assert out16.shape == (2, N)
    write_wav(os.path.join(args.out, 'music.wav'), out16)
    for k, v in stems.items():
        write_wav(os.path.join(args.out, 'stems', f'{k}.wav'), STEM_GAIN[k] * v * stem_scale, 'PCM_24')

    alt = None
    if not args.fast:
        mx2 = Mixer(exclude={'crash240'})
        build_score(mx2)
        stems2 = render_stems(mx2)
        m2, _, _ = master_chain(premaster(stems2), gain_db=gain_db)
        write_wav(os.path.join(args.out, 'music-drop-nocrash.wav'), finalize(m2))
        alt = 'music-drop-nocrash.wav'

    y = out16.astype(np.float64) / 32768
    meter = pyln.Meter(SR)
    lufs = meter.integrated_loudness(y.T)
    tp = 20 * math.log10(true_peak(y) + 1e-12)
    markers = {
        'file': 'music.wav',
        'generator': 'tools/music/compose.py (deterministic; seeds = crc32(instrument, sample position))',
        'sampleRate': SR, 'channels': 2, 'bitDepth': 16, 'fps': FPS, 'samplesPerFrame': SPF,
        'lengthFrames': NFR, 'lengthSamples': N, 'lengthSeconds': N / SR,
        'bpm': 120, 'meter': '4/4', 'key': 'A minor', 'beatFrames': BEAT, 'barFrames': BAR,
        'master': {'integratedLUFS': round(lufs, 2), 'truePeakDBTP': round(tp, 2), 'gainDb': round(gain_db, 2),
                   'limiterMaxGainReductionDb': round(-20 * math.log10(float(np.min(gr)) + 1e-12), 2)},
        'ducking': 'Not baked in. src/shared/audio.ts DUCKS (105, 180, 240, 510, 525, 540, 1035, 1380) apply on top.',
        'alternates': {'dropWithoutCrash': alt},
        'stems': {k: f'stems/{k}.wav' for k in STEMS},
        'stemsNote': f'Pre-master, 24-bit, summed they equal the pre-master mix x {stem_scale:.4f}. Same length as music.wav.',
        'sections': [],
        'events': [],
    }
    for s in json.load(open(os.path.join(PROJ, 'brief', 'storyboard.json')))['music']['sections']:
        f0 = (s['startBar'] - 1) * BAR
        markers['sections'].append({'name': s['name'], 'bars': [s['startBar'], s['startBar'] + s['bars'] - 1],
                                    'frames': [f0, f0 + s['bars'] * BAR - 1], 'energy': s['energy'],
                                    'startTime': f0 / FPS})
    for e in sorted(EVENTS, key=lambda e: e['frame']):
        e = dict(e)
        e['time'] = round(e['frame'] / FPS, 4)
        e['sample'] = S(e['frame'])
        e['barBeat'] = f"{int(e['frame'] // BAR) + 1}.{int((e['frame'] % BAR) // BEAT) + 1}"
        markers['events'].append(e)
    with open(os.path.join(args.out, 'markers.json'), 'w') as fh:
        json.dump(markers, fh, indent=1, ensure_ascii=False)
    print(f'music.wav {N} samples, {lufs:.2f} LUFS, {tp:.2f} dBTP, master gain {gain_db:+.2f} dB')


if __name__ == '__main__':
    main()
