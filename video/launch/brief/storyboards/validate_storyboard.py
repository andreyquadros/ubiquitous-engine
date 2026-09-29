#!/usr/bin/env python3
"""Validate a launch-film storyboard JSON against the brief's hard rules and the real assets.
Usage: validate_storyboard.py path/to/draft.json"""
import json, os, re, sys

ROOT = '/home/user/ubiquitous-engine/video/launch'
PUB = ROOT + '/public'
path = sys.argv[1] if len(sys.argv) > 1 else ROOT + '/brief/storyboards/draft-c.json'

errors, warns = [], []
def err(m): errors.append(m)
def warn(m): warns.append(m)

doc = json.load(open(path, encoding='utf-8'))  # 1. parses
UI = {e['file']: e for e in json.load(open(PUB + '/ui/ui-manifest.json'))}
HOT = {f: {h['name'] for h in e['hotspots']} for f, e in UI.items()}
UBI = {c['dir']: c for c in json.load(open(PUB + '/ubi/ubi-manifest.json'))['clips']}
SFXM = {e['file']: e for e in json.load(open(PUB + '/audio/sfx/sfx-manifest.json'))}

for k, v in (('fps', 30), ('width', 1920), ('height', 1080), ('bpm', 120)):
    if doc.get(k) != v: err(f'top-level {k} = {doc.get(k)} (expected {v})')
for k in ('title', 'angle', 'durationInFrames', 'music', 'scenes'):
    if k not in doc: err(f'missing top-level {k}')

scenes = doc['scenes']
total = doc['durationInFrames']
secs = total / 30
if not (44 <= secs <= 52): err(f'duration {secs:.2f} s outside 44–52 s')
if not (12 <= len(scenes) <= 22): err(f'{len(scenes)} scenes (12–22 required)')

REQ = ['id', 'startFrame', 'durationInFrames', 'act', 'purpose', 'shotTypes', 'copy', 'visual', 'assets', 'camera',
       'cursor', 'motion', 'transitionIn', 'transitionOut', 'sfx', 'readingCheck']
ACTS = {'hook', 'problem', 'reveal', 'features', 'proof', 'cta'}
TTYPES = {'cut', 'whip-left', 'whip-right', 'zoom-through', 'mask-circle', 'mask-diagonal', 'flash', 'blur-dissolve', 'match-cut'}
MODES = {'slam', 'stagger-words', 'mask-up', 'typewriter', 'static'}
ROLES = {'hero', 'headline', 'sub', 'kicker', 'label', 'ui-caption'}
BANNED_COPY = [r'1\s*[–-]\s*9', r'\b1 a 9\b', r'gr[áa]tis', r'offline', r'\bassine\b', r'\bcompre\b', r'powered', r'parceria',
               r'UBIQX', r'Ubiqx', r'sem juros', r'\btrial\b', r'criptograf', r'nunca erra', r'100 ?%']
BANNED_FILES = {'brand/og.png'}
PRIVACY_FILES = {'ui/timeline.png', 'ui/timeline-selected.png', 'ui/timeline-reclassify.png', 'ui/reports-generated.png'}

# 2. tiling
pos = 0
for i, s in enumerate(scenes):
    for k in REQ:
        if k not in s: err(f'{s.get("id")}: missing {k}')
    if s['startFrame'] != pos: err(f'{s["id"]}: starts {s["startFrame"]}, expected {pos} (gap/overlap)')
    if s['startFrame'] % 15: warn(f'{s["id"]}: start {s["startFrame"]} not on a beat')
    if s['act'] not in ACTS: err(f'{s["id"]}: bad act {s["act"]}')
    pos = s['startFrame'] + s['durationInFrames']
if pos != total: err(f'scenes end at {pos}, durationInFrames = {total}')
if sum(s['durationInFrames'] for s in scenes) != total: err('sum of durations != durationInFrames')

# 3. transitions pair up
types = set()
for a, b in zip(scenes, scenes[1:]):
    ta, tb = a['transitionOut']['type'], b['transitionIn']['type']
    if ta != tb: err(f'{a["id"]}→{b["id"]}: transitionOut {ta} != transitionIn {tb}')
for s in scenes:
    for k in ('transitionIn', 'transitionOut'):
        t = s[k]
        if t['type'] not in TTYPES: err(f'{s["id"]}: {k} type {t["type"]} not allowed')
        if not (0 <= t['frames'] <= s['durationInFrames']): err(f'{s["id"]}: {k} frames out of range')
        types.add(t['type'])
if len(types) > 6: err(f'{len(types)} distinct transition types (> 6)')

# 4. copy + reading rule
for s in scenes:
    d = s['durationInFrames']
    groups = {}
    for i, c in enumerate(s['copy']):
        for k in ('text', 'role', 'inFrame', 'outFrame', 'emphasis', 'mode'):
            if k not in c: err(f'{s["id"]}: copy missing {k}')
        if c['mode'] not in MODES: err(f'{s["id"]}: copy mode {c["mode"]}')
        if c['role'] not in ROLES: err(f'{s["id"]}: copy role {c["role"]}')
        land = c.get('landFrame', c['inFrame'])
        if not (0 <= c['inFrame'] <= land <= c['outFrame'] <= d): err(f'{s["id"]}: copy frames out of order/range: {c["text"]}')
        for e in c['emphasis']:
            if e not in c['text']: err(f'{s["id"]}: emphasis "{e}" not in "{c["text"]}"')
        for pat in BANNED_COPY:
            if re.search(pat, c['text'], re.I if pat not in ('UBIQX', 'Ubiqx') else 0): err(f'{s["id"]}: banned pattern {pat} in "{c["text"]}"')
        groups.setdefault(c.get('group') or f'solo{i}', []).append(c)
    for g, items in groups.items():
        chars = sum(len(c['text']) for c in items)
        need = max(24, 2 * chars)
        held = min(c['outFrame'] for c in items) - max(c.get('landFrame', c['inFrame']) for c in items)
        if held < need: err(f'{s["id"]}: reading rule fails for {[c["text"] for c in items]}: held {held} < {need}')

# 5. assets
for s in scenes:
    uifiles = []
    for a in s['assets']:
        kind, ref = a['kind'], a['ref']
        if kind == 'ui':
            f = ref.split('/', 1)[1]
            if not os.path.isfile(f'{PUB}/{ref}'): err(f'{s["id"]}: missing UI file {ref}')
            if f not in HOT: err(f'{s["id"]}: {ref} not in ui-manifest'); continue
            if ref in PRIVACY_FILES: warn(f'{s["id"]}: uses privacy-flagged capture {ref}')
            uifiles.append(f)
            for h in a.get('hotspots', []):
                if h not in HOT[f]: err(f'{s["id"]}: hotspot {h} not in {f}')
        elif kind == 'ubi':
            if not os.path.isdir(f'{PUB}/{ref}') or ref not in UBI: err(f'{s["id"]}: missing UBI clip dir {ref}')
        elif kind == 'brand':
            if not os.path.isfile(f'{PUB}/{ref}'): err(f'{s["id"]}: missing brand file {ref}')
            if ref in BANNED_FILES or ref.startswith('brand/ai/'): err(f'{s["id"]}: banned brand asset {ref}')
        elif kind == 'sfx':
            if not os.path.isfile(f'{PUB}/audio/sfx/{os.path.basename(ref)}'): err(f'{s["id"]}: missing sfx {ref}')
        elif kind != 'none':
            err(f'{s["id"]}: unknown asset kind {kind}')
    s['_ui'] = uifiles
    # camera
    last = {}
    for c in s['camera']:
        if not (0 <= c['atFrame'] < s['durationInFrames']): err(f'{s["id"]}: camera atFrame {c["atFrame"]} outside scene')
        layer = (c.get('layer'), c.get('file'))
        if c['atFrame'] < last.get(layer, -1): err(f'{s["id"]}: camera keys not in order for {layer}')
        last[layer] = c['atFrame']
        f = c.get('file')
        if f:
            fn = f.split('/', 1)[1]
            if fn not in s['_ui']: err(f'{s["id"]}: camera file {f} not declared in assets')
            if c['target'] not in HOT.get(fn, set()) | {'full'}: err(f'{s["id"]}: camera target {c["target"]} not a hotspot of {fn}')
            rt = c.get('readTarget')
            if rt and not rt['ok']: err(f'{s["id"]}: readTarget {rt["text"]} only {rt["onScreenPx"]} px')
            if c.get('upsample', 0) > 1.4: err(f'{s["id"]}: upsample {c["upsample"]} > 1.4')
            if c.get('upsample', 0) > 1.15 and not rt: warn(f'{s["id"]}: upsample {c["upsample"]} at f{c["atFrame"]} without a read target')
        elif c['target'] not in ('canvas', 'full'):
            err(f'{s["id"]}: camera target {c["target"]} without file')
    # cursor
    for k in s['cursor']:
        to = k['to']
        if to.startswith(('rest:', 'comp:')): continue
        if not any(to in HOT[f] for f in s['_ui']): err(f'{s["id"]}: cursor target {to} not a hotspot of {s["_ui"]}')
    # ubi track
    for u in s.get('ubiTrack', []):
        if u['clip'] == 'hidden': continue
        if u['clip'] not in UBI: err(f'{s["id"]}: ubiTrack clip {u["clip"]} unknown'); continue
        n = UBI[u['clip']]['frames']
        span = u['to'] - u['from']
        if not u.get('loop') and not u.get('hold') and u['startIndex'] + span >= n:
            err(f'{s["id"]}: ubiTrack {u["clip"]} index {u["startIndex"]}+{span} ≥ {n} frames')
        if not (0 <= u['from'] <= u['to'] < s['durationInFrames']): err(f'{s["id"]}: ubiTrack frames out of range')
        if u['clip'] not in [a['ref'] for a in s['assets'] if a['kind'] == 'ubi']: err(f'{s["id"]}: ubiTrack clip {u["clip"]} not in assets')
    # sfx
    for e in s['sfx']:
        if not os.path.isfile(f'{PUB}/audio/sfx/{e["ref"]}'): err(f'{s["id"]}: missing sfx file {e["ref"]}')
        if e['ref'] not in SFXM: err(f'{s["id"]}: sfx {e["ref"]} not in sfx-manifest')

# 6. SFX overlap budget (absolute timeline, by file duration)
spans = []
for s in scenes:
    for e in s['sfx']:
        st = s['startFrame'] + e.get('fileStartFrame', e['atFrame'])
        dur = round(SFXM[e['ref']]['duration_s'] * 30)
        spans.append((st, st + dur, e['ref'], s['id']))
for f in range(total):
    live = [x for x in spans if x[0] <= f < x[1]]
    if len(live) > 3: err(f'frame {f}: {len(live)} SFX overlap: {[x[2] for x in live]}'); break
beats = total // 15
if len(spans) > beats: warn(f'{len(spans)} SFX cues for {beats} beats')

# 7. music map
m = doc['music']
bar = 1
for sec in m['sections']:
    if sec['startBar'] != bar: err(f'music section {sec["name"]} starts bar {sec["startBar"]}, expected {bar}')
    bar += sec['bars']
    for e in sec['events']:
        if not (0 <= e['frame'] < total): err(f'music event {e} outside film')
if (bar - 1) * 60 != total: err(f'music sections cover {(bar - 1) * 60} frames, film is {total}')
if 'key' not in m: err('music.key missing')

print(f'{path}\n  {len(scenes)} scenes, {total} frames = {secs:.1f} s, transitions: {sorted(types)}, SFX cues: {len(spans)}')
for w in warns: print('  WARN', w)
for e in errors: print('  ERROR', e)
print('  VALID' if not errors else f'  {len(errors)} error(s)')
sys.exit(1 if errors else 0)
