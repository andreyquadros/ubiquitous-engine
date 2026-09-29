#!/usr/bin/env python3
"""Validate the launch-film storyboard JSON against the brief's hard rules and the real assets.

Usage:
    python3 tools/validate-storyboard.py [path/to/storyboard.json] [--quiet]

Default path: brief/storyboard.json (relative to video/launch/). Exit code 0 = no errors.
Warnings never fail the run; errors do.

Checks
  1. JSON parses; top-level fields; fps/size/bpm; duration 44–52 s in whole bars.
  2. Scenes tile exactly (no gaps/overlaps), durations sum, starts on beats (warn otherwise).
  3. Transitions: out/in types pair up, halves fit, ≤ 6 types, whip ≤ 3, flash ≤ 2,
     blur-dissolve ≤ 2, match-cut boundaries ≤ 4.
  4. Copy: roles/modes, frame order, emphasis substrings, reading rule
     hold ≥ max(24, 2 × chars) per line; groups that land together ("together": sum of their
     characters from the last landing to the first exit); and a per-scene reading budget
     (sum of all characters, from the first landing to the last exit).
  5. Forbidden strings in the whole JSON (key ranges, purchase verbs, endorsement wording,
     provider logo files, the OG image) and forbidden claims in on-screen copy.
  6. Assets: every UI file + hotspot exists in ui-manifest.json and on disk; privacy-flagged
     captures unused; brand files exist; camera/cursor/spotlight/patch targets resolve against
     the scene's declared captures.
  7. UI legibility: every camera key on a capture carries its bitmap scale; scale ≤ 1.40;
     every readTarget recomputed (captureFontPx × zoom × screenWidth / imageWidth × lift) ≥ 36 px,
     and a vector re-set wherever scale > 1.15; a key above 1.15 with no readTarget must say why
     (textureReason).
  8. Claim safety: every sensitive region (key-range legend/hint, model names, continuity counts,
     mail rows) that falls inside any camera key's visible image rect is covered by an active
     solid patch (image space) or a solid scrim (screen space).
  9. UBI: clip dirs exist, requested indices fit the clip lengths (and the PNGs exist), frames
     inside the scene, clips declared in assets.
 10. SFX: files exist and are in sfx-manifest.json; fileStartFrame = atFrame − hit offset (±1 f);
     ≤ 3 cues sounding at once; nothing sounds inside a declared silence; tail ends ≤ last frame.
 11. Music map: sections start at bar 1 and tile the film in whole bars; events inside the film;
     a downbeat hit on frame 0; key given.
 12. Pacing: shot count 28–38 (warn), shot lengths 4–120 f (end card ≤ 180).
 13. End card: tagline, CTA, platform line present and ≥ 90 f fully legible after the last landing.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
PUB = os.path.join(ROOT, 'public')

args = [a for a in sys.argv[1:] if not a.startswith('--')]
QUIET = '--quiet' in sys.argv
path = args[0] if args else os.path.join(ROOT, 'brief', 'storyboard.json')

errors, warns = [], []


def err(m):
    errors.append(m)


def warn(m):
    warns.append(m)


# ------------------------------------------------------------------ 1. parse
try:
    raw = open(path, encoding='utf-8').read()
    doc = json.loads(raw)
except Exception as e:  # noqa: BLE001
    print(f'{path}\n  ERROR JSON does not parse: {e}')
    sys.exit(1)

UI = {e['file']: e for e in json.load(open(os.path.join(PUB, 'ui', 'ui-manifest.json'), encoding='utf-8'))}
HOT = {f: {h['name']: h for h in e['hotspots']} for f, e in UI.items()}
UBI = {c['dir']: c for c in json.load(open(os.path.join(PUB, 'ubi', 'ubi-manifest.json'), encoding='utf-8'))['clips']}
SFXM = {e['file']: e for e in json.load(open(os.path.join(PUB, 'audio', 'sfx', 'sfx-manifest.json'), encoding='utf-8'))}

FPS, BEAT, BAR = 30, 15, 60
W, H = 1920, 1080

for k, v in (('fps', 30), ('width', 1920), ('height', 1080), ('bpm', 120)):
    if doc.get(k) != v:
        err(f'top-level {k} = {doc.get(k)} (expected {v})')
for k in ('title', 'angle', 'durationInFrames', 'music', 'scenes'):
    if k not in doc:
        err(f'missing top-level {k}')
if errors:
    for e in errors:
        print('  ERROR', e)
    sys.exit(1)

scenes = doc['scenes']
TOTAL = doc['durationInFrames']
secs = TOTAL / FPS
if not (44 <= secs <= 52):
    err(f'duration {secs:.2f} s outside 44–52 s')
if TOTAL % BAR:
    err(f'duration {TOTAL} f is not a whole number of bars')
if not (12 <= len(scenes) <= 22):
    err(f'{len(scenes)} scenes (12–22 expected)')

REQ = ['id', 'startFrame', 'durationInFrames', 'act', 'purpose', 'shotTypes', 'copy', 'visual', 'assets', 'camera',
       'cursor', 'motion', 'transitionIn', 'transitionOut', 'sfx', 'readingCheck']
ACTS = {'hook', 'problem', 'reveal', 'features', 'proof', 'cta'}
TTYPES = {'cut', 'whip-left', 'whip-right', 'zoom-through', 'mask-circle', 'mask-diagonal', 'flash', 'blur-dissolve', 'match-cut'}
MODES = {'slam', 'stagger-words', 'mask-up', 'typewriter', 'static'}
ROLES = {'hero', 'headline', 'sub', 'kicker', 'label', 'ui-caption'}
PRIVACY_FILES = {'timeline.png', 'timeline-selected.png', 'timeline-reclassify.png', 'reports-generated.png'}

# ------------------------------------------------------------------ 2. tiling
pos = 0
ids = set()
for s in scenes:
    for k in REQ:
        if k not in s:
            err(f'{s.get("id")}: missing {k}')
    if s['id'] in ids:
        err(f'duplicate scene id {s["id"]}')
    ids.add(s['id'])
    if s['startFrame'] != pos:
        err(f'{s["id"]}: starts {s["startFrame"]}, expected {pos} (gap/overlap)')
    if s['startFrame'] % BEAT:
        warn(f'{s["id"]}: start {s["startFrame"]} not on a beat')
    if s['act'] not in ACTS:
        err(f'{s["id"]}: bad act {s["act"]}')
    if s['durationInFrames'] <= 0:
        err(f'{s["id"]}: non-positive duration')
    pos = s['startFrame'] + s['durationInFrames']
if pos != TOTAL:
    err(f'scenes end at {pos}, durationInFrames = {TOTAL}')
if sum(s['durationInFrames'] for s in scenes) != TOTAL:
    err('sum of scene durations != durationInFrames')

# ------------------------------------------------------------------ 3. transitions
types, counts = set(), {}
for a, b in zip(scenes, scenes[1:]):
    ta, tb = a['transitionOut']['type'], b['transitionIn']['type']
    if ta != tb:
        err(f'{a["id"]}→{b["id"]}: transitionOut {ta} != transitionIn {tb}')
    counts[ta] = counts.get(ta, 0) + 1
for s in scenes:
    for k in ('transitionIn', 'transitionOut'):
        t = s[k]
        if t['type'] not in TTYPES:
            err(f'{s["id"]}: {k} type {t["type"]} not allowed')
        if not (0 <= t.get('frames', -1) <= s['durationInFrames']):
            err(f'{s["id"]}: {k} frames {t.get("frames")} out of range')
        types.add(t['type'])
if len(types) > 6:
    err(f'{len(types)} distinct transition types (> 6)')
whips = counts.get('whip-left', 0) + counts.get('whip-right', 0)
if whips > 3:
    err(f'{whips} whips (≤ 3)')
if counts.get('flash', 0) > 2:
    err(f'{counts["flash"]} flash transitions (≤ 2)')
if counts.get('blur-dissolve', 0) > 2:
    err(f'{counts["blur-dissolve"]} blur dissolves (≤ 2)')
if counts.get('match-cut', 0) > 4:
    err(f'{counts["match-cut"]} match cuts (≤ 4)')
if counts.get('cut', 0) < (len(scenes) - 1) / 2:
    warn('fewer than half of the scene boundaries are hard cuts')

# ------------------------------------------------------------------ 4. copy + reading rule
BANNED_COPY = [
    (r'(?<!\d)1\s*[–—-]\s*9(?!\d)', 'key range'), (r'(?<!\d)1 a 9(?!\d)', 'key range'),
    (r'gr[áa]tis', 'free'), (r'\boffline\b', 'offline'), (r'\bassin', 'purchase verb'), (r'\bcompr[ea]\b', 'purchase verb'),
    (r'powered', 'endorsement'), (r'parceria', 'endorsement'), (r'\boficial\b', 'endorsement'), (r'sem juros', 'installments'),
    (r'\btrial\b', 'trial'), (r'criptograf', 'encryption claim'), (r'nunca erra', 'accuracy claim'), (r'100 ?%', 'accuracy claim'),
    (r'nada sai', 'nothing-leaves claim'), (r'IA local', 'local-AI claim'), (r'sem internet', 'offline claim'),
    (r'(?i)\bsonnet\b|\bhaiku\b|\bgpt-|grok-\d|\bopus\b', 'model version name'),
]
CASE_SENSITIVE = [(r'UBIQX|Ubiqx', 'brand casing')]


def need(chars):
    return max(24, 2 * chars)


end_tagline = end_cta = end_platforms = False
for s in scenes:
    d = s['durationInFrames']
    groups = {}
    for i, c in enumerate(s['copy']):
        for k in ('text', 'role', 'inFrame', 'outFrame', 'emphasis', 'mode'):
            if k not in c:
                err(f'{s["id"]}: copy missing {k}')
        if c['mode'] not in MODES:
            err(f'{s["id"]}: copy mode {c["mode"]}')
        if c['role'] not in ROLES:
            err(f'{s["id"]}: copy role {c["role"]}')
        land = c.get('landFrame', c['inFrame'])
        if not (0 <= c['inFrame'] <= land <= c['outFrame'] <= d):
            err(f'{s["id"]}: copy frames out of order/range: {c["text"]!r}')
        for e in c['emphasis']:
            if e not in c['text']:
                err(f'{s["id"]}: emphasis {e!r} not in {c["text"]!r}')
        for pat, why in BANNED_COPY:
            if re.search(pat, c['text'], re.I):
                err(f'{s["id"]}: forbidden {why} ({pat}) in copy {c["text"]!r}')
        for pat, why in CASE_SENSITIVE:
            if re.search(pat, c['text']):
                err(f'{s["id"]}: {why} in copy {c["text"]!r}')
        n = len(c['text'])
        held = c['outFrame'] - land
        if held < need(n):
            err(f'{s["id"]}: reading rule fails for {c["text"]!r}: held {held} < {need(n)}')
        abs_out = s['startFrame'] + c['outFrame']
        if c['outFrame'] != d and abs_out % BEAT:
            warn(f'{s["id"]}: {c["text"]!r} exits at abs {abs_out}, not on a beat')
        if c.get('group'):
            groups.setdefault(c['group'], []).append(c)
        t = c['text'].replace(' ', ' ')
        if s['act'] == 'cta':
            end_tagline |= t == 'Retome o controle do seu dia.'
            end_cta |= t == 'Baixe em ubiqx.com.br'
            end_platforms |= t == 'macOS · Windows · Linux'
    for g, items in groups.items():
        modes = {c.get('groupMode', 'together') for c in items}
        if len(modes) != 1:
            err(f'{s["id"]}: group {g} mixes groupModes {modes}')
        mode = modes.pop()
        tot = need(sum(len(c['text']) for c in items))
        if mode == 'together':
            av = min(c['outFrame'] for c in items) - max(c.get('landFrame', c['inFrame']) for c in items)
        elif mode == 'sequential':
            av = max(c['outFrame'] for c in items) - min(c.get('landFrame', c['inFrame']) for c in items)
        else:
            err(f'{s["id"]}: group {g} unknown groupMode {mode}')
            continue
        if av < tot:
            err(f'{s["id"]}: reading rule fails for group {g} ({mode}): {av} < {tot}')
    # scene reading budget: everything on screen in this scene, read in sequence
    if len(s['copy']) > 1:
        tot = need(sum(len(c['text']) for c in s['copy']))
        av = max(c['outFrame'] for c in s['copy']) - min(c.get('landFrame', c['inFrame']) for c in s['copy'])
        if av < tot:
            err(f'{s["id"]}: scene reading budget fails: {av} f between first landing and last exit < {tot} f')

# ------------------------------------------------------------------ 5. forbidden strings anywhere in the JSON
FORBIDDEN_ANYWHERE = [
    (r'(?<!\d)1\s*[–—-]\s*9(?!\d)', 'key range 1–9'), (r'(?<![\d\w])1 a 9(?!\d)', 'key range 1 a 9'),
    (r'\bAssine\b', 'Assine'), (r'\bCompre\b', 'Compre'), (r'(?i)parceria', 'parceria'), (r'(?i)powered by', 'powered by'),
    (r'(?i)og\.png', 'OG image'), (r'(?i)brand/ai/', 'provider logo path'),
    (r'(?i)(anthropic|claude(-color)?|openai|xai|grok|gemini)\.svg', 'provider logo file'),
]
for pat, why in FORBIDDEN_ANYWHERE:
    for m in re.finditer(pat, raw):
        ctx = raw[max(0, m.start() - 40):m.end() + 40].replace('\n', ' ')
        err(f'forbidden string ({why}) in JSON: …{ctx}…')

# ------------------------------------------------------------------ 6–8. assets, camera, cursor, legibility, claim safety
# Sensitive regions (image px), measured from the PNG pixels (ink boxes).
SENSITIVE = {
    'review-queue-selected.png': [('key-range hint', (636, 405, 1210, 22)), ('keys legend', (2144, 813, 622, 18)),
                                  ('mail row', (635, 1051, 485, 55))],
    'review-after-assign.png': [('key-range hint', (636, 405, 1200, 22)), ('keys legend', (2144, 1023, 622, 18)),
                                ('mail row', (635, 939, 485, 55))],
    'review-settled-expanded.png': [('keys legend', (2144, 669, 622, 18))],
    'review-confirmed.png': [('keys legend', (2144, 669, 622, 18))],
    'review-done.png': [('keys legend', (2144, 1023, 622, 18)), ('resolved count', (900, 847, 758, 67)),
                        ('settled header count', (547, 1147, 1457, 26))],
    'review-details.png': [],
    'reports.png': [('model version (meta 1)', (651, 440, 1193, 23)), ('model version (meta 2)', (651, 1700, 1193, 23)),
                    ('mail row', (1469, 1455, 394, 35))],
    'settings-ai.png': [('model names', (1146, 961, 1644, 312))],
    'settings-ai-ubi.png': [('model names', (1146, 761, 1644, 312))],
}
MARGIN = 40  # image px of slack for tilt/float/drift


def rect_of(r):
    return (r['x'], r['y'], r['w'], r['h']) if isinstance(r, dict) else tuple(r)


def contains(outer, inner):
    ox, oy, ow, oh = outer
    ix, iy, iw, ih = inner
    return ox <= ix and oy <= iy and ox + ow >= ix + iw and oy + oh >= iy + ih


def intersects(a, b):
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax < bx + bw and bx < ax + aw and ay < by + bh and by < ay + ah


shot_starts = []
for s in scenes:
    sid, d = s['id'], s['durationInFrames']
    uifiles = []
    ubi_decl = set()
    for a in s['assets']:
        kind, ref = a['kind'], a['ref']
        if kind == 'ui':
            f = ref.split('/', 1)[1] if ref.startswith('ui/') else ref
            if not os.path.isfile(os.path.join(PUB, 'ui', f)):
                err(f'{sid}: missing UI file {ref}')
            if f not in HOT:
                err(f'{sid}: {ref} not in ui-manifest')
                continue
            if f in PRIVACY_FILES:
                err(f'{sid}: uses privacy-flagged capture {ref}')
            uifiles.append(f)
            for h in a.get('hotspots', []):
                if h not in HOT[f]:
                    err(f'{sid}: hotspot {h} not in {f}')
        elif kind == 'ubi':
            if ref not in UBI or not os.path.isdir(os.path.join(PUB, ref)):
                err(f'{sid}: missing UBI clip dir {ref}')
            ubi_decl.add(ref)
        elif kind == 'brand':
            if not os.path.isfile(os.path.join(PUB, ref)):
                err(f'{sid}: missing brand file {ref}')
        elif kind == 'sfx':
            if not os.path.isfile(os.path.join(PUB, 'audio', 'sfx', os.path.basename(ref))):
                err(f'{sid}: missing sfx {ref}')
        elif kind != 'none':
            err(f'{sid}: unknown asset kind {kind}')

    patches = s.get('patches', [])
    for p in patches:
        f = p['file'].split('/', 1)[1]
        if f not in uifiles:
            err(f'{sid}: patch on {p["file"]} not declared in assets')
            continue
        x, y, w, h = rect_of(p['rect'])
        if x < 0 or y < 0 or x + w > UI[f]['width'] or y + h > UI[f]['height']:
            err(f'{sid}: patch {p["rect"]} outside {f}')
        if not re.fullmatch(r'#[0-9a-fA-F]{6}', p.get('color', '')):
            err(f'{sid}: patch colour {p.get("color")!r} is not a #rrggbb')
    scrims = [sc for sc in s.get('scrims', []) if sc.get('solid')]

    for sp in s.get('spotlights', []):
        f = sp['file'].split('/', 1)[1]
        if f not in uifiles:
            err(f'{sid}: spotlight file {sp["file"]} not declared in assets')
        elif 'hotspot' in sp and sp['hotspot'] not in HOT[f]:
            err(f'{sid}: spotlight hotspot {sp["hotspot"]} not in {f}')

    # camera
    last = {}
    for c in s['camera']:
        at = c['atFrame']
        if not (0 <= at < d):
            err(f'{sid}: camera atFrame {at} outside scene')
        layer = (c.get('layer'), c.get('file'))
        if at < last.get(layer, -1):
            err(f'{sid}: camera keys not in order for {layer}')
        last[layer] = at
        fref = c.get('file')
        if not fref:
            if c['target'] not in ('canvas', 'full'):
                err(f'{sid}: camera target {c["target"]} without file')
            continue
        f = fref.split('/', 1)[1]
        if f not in uifiles:
            err(f'{sid}: camera file {fref} not declared in assets')
            continue
        if c['target'] not in set(HOT[f]) | {'full'}:
            err(f'{sid}: camera target {c["target"]} not a hotspot of {f}')
        img = UI[f]
        sw = c.get('screenWidth')
        if not sw:
            err(f'{sid}: camera key f{at} on {f} has no screenWidth')
            continue
        s_eff = c['zoom'] * sw / img['width']
        if abs(s_eff - c.get('upsample', -1)) > 0.01:
            err(f'{sid}: camera f{at} upsample {c.get("upsample")} != zoom×width/imageWidth = {s_eff:.3f}')
        if s_eff > 1.40 + 1e-9:
            err(f'{sid}: camera f{at} bitmap scale {s_eff:.3f} > 1.40')
        if 'readTarget' not in c:
            err(f'{sid}: camera f{at} on {f} must declare readTarget (object or null)')
        rt = c.get('readTarget')
        if not rt and s_eff > 1.15 + 1e-9 and not c.get('textureReason'):
            err(f'{sid}: camera f{at} at bitmap scale {s_eff:.2f} has no readTarget and no textureReason')
        if rt:
            on = rt['captureFontPx'] * s_eff * rt.get('lift', 1)
            if on < 36 - 1e-6:
                err(f'{sid}: readTarget {rt["text"]!r} only {on:.1f} px on screen (< 36)')
            if abs(on - rt.get('onScreenPx', -1)) > 0.2:
                err(f'{sid}: readTarget {rt["text"]!r} onScreenPx {rt.get("onScreenPx")} != recomputed {on:.1f}')
            if s_eff > 1.15 and rt.get('crisp') != 'vector':
                err(f'{sid}: readTarget {rt["text"]!r} at bitmap scale {s_eff:.2f} needs a vector re-set')
            if rt.get('lift', 1) > 1.1:
                err(f'{sid}: readTarget lift {rt["lift"]} > 1.10')
        # visible image rect for this key (tilt/float ignored → MARGIN)
        fx, fy = (c['focus']['x'], c['focus']['y']) if 'focus' in c else (
            (HOT[f][c['target']]['x'] + HOT[f][c['target']]['w'] / 2, HOT[f][c['target']]['y'] + HOT[f][c['target']]['h'] / 2)
            if c['target'] != 'full' else (img['width'] / 2, img['height'] / 2))
        ax, ay = (c['anchor']['x'], c['anchor']['y']) if 'anchor' in c else (W / 2, H / 2)
        vis = (fx - ax / s_eff - MARGIN, fy - ay / s_eff - MARGIN, W / s_eff + 2 * MARGIN, H / s_eff + 2 * MARGIN)
        for name, reg in SENSITIVE.get(f, []):
            if not intersects(vis, reg):
                continue
            covered = any(p['file'].split('/', 1)[1] == f and p['from'] <= at <= p['to'] and contains(rect_of(p['rect']), reg)
                          for p in patches)
            if not covered:
                rx, ry, rw, rh = reg
                comp = (ax + (rx - fx) * s_eff, ay + (ry - fy) * s_eff, rw * s_eff, rh * s_eff)
                covered = any(sc['from'] <= at <= sc['to'] and contains(rect_of(sc['rect']), comp) for sc in scrims) or \
                    not intersects((0, 0, W, H), comp)
            if not covered:
                err(f'{sid}: camera f{at} frames the {name} of {f} {reg} with no solid patch/scrim over it')

    # cursor
    for k in s['cursor']:
        to = k['to']
        if not (0 <= k['atFrame'] < d):
            err(f'{sid}: cursor atFrame {k["atFrame"]} outside scene')
        if to.startswith(('rest:', 'comp:')):
            continue
        if not any(to in HOT[f] for f in uifiles):
            err(f'{sid}: cursor target {to} not a hotspot of {uifiles}')

    # UBI track
    for u in s.get('ubiTrack', []):
        if not (0 <= u['from'] <= u['to'] < d):
            err(f'{sid}: ubiTrack {u["clip"]} frames {u["from"]}–{u["to"]} out of range')
        if u['clip'] == 'hidden':
            continue
        if u['clip'] not in UBI:
            err(f'{sid}: ubiTrack clip {u["clip"]} unknown')
            continue
        if u['clip'] not in ubi_decl:
            err(f'{sid}: ubiTrack clip {u["clip"]} not in assets')
        n = UBI[u['clip']]['frames']
        span = u['to'] - u['from']
        first = u['startIndex']
        lastidx = first if u.get('hold') else first + span
        if first < 0 or first >= n:
            err(f'{sid}: ubiTrack {u["clip"]} startIndex {first} outside 0–{n - 1}')
        if not u.get('loop') and lastidx >= n:
            err(f'{sid}: ubiTrack {u["clip"]} needs index {lastidx} ≥ {n} frames')
        for idx in {first, min(lastidx, n - 1)}:
            fp = os.path.join(PUB, u['clip'], f'{idx:04d}.png')
            if not os.path.isfile(fp):
                err(f'{sid}: missing UBI frame {fp}')

    # shots
    sh = s.get('shots', [0])
    if not sh or sh[0] != 0 or sorted(sh) != sh or any(not (0 <= x < d) for x in sh):
        err(f'{sid}: shots must start at 0, be ascending and inside the scene')
    shot_starts += [s['startFrame'] + x for x in sh]

# ------------------------------------------------------------------ 10. SFX
spans = []
for s in scenes:
    for e in s['sfx']:
        ref = e['ref']
        if not os.path.isfile(os.path.join(PUB, 'audio', 'sfx', ref)):
            err(f'{s["id"]}: missing sfx file {ref}')
            continue
        if ref not in SFXM:
            err(f'{s["id"]}: sfx {ref} not in sfx-manifest')
            continue
        m = SFXM[ref]
        off = m['hit_offset_s'] * FPS
        fs = e.get('fileStartFrame', e['atFrame'])
        if abs(fs - (e['atFrame'] - off)) > 1.01:
            err(f'{s["id"]}: {ref} fileStartFrame {fs} but hit at {e["atFrame"]} − offset {off:.1f} f')
        st = s['startFrame'] + fs
        dur = round(m['duration_s'] * FPS)
        spans.append((st, st + dur, ref, s['id']))
        if st < 0:
            err(f'{s["id"]}: {ref} starts before frame 0')
        if st + dur > TOTAL:
            err(f'{s["id"]}: {ref} rings past the last frame ({st + dur} > {TOTAL})')
worst = (0, None)
for f in range(TOTAL):
    live = [x for x in spans if x[0] <= f < x[1]]
    if len(live) > worst[0]:
        worst = (len(live), f, [x[2] for x in live])
if worst[0] > 3:
    err(f'frame {worst[1]}: {worst[0]} SFX overlap: {worst[2]}')
if len(spans) > TOTAL // BEAT:
    warn(f'{len(spans)} SFX cues for {TOTAL // BEAT} beats')

# ------------------------------------------------------------------ 11. music map
m = doc['music']
if 'key' not in m:
    err('music.key missing')
bar = 1
silences = []
downbeat0 = False
for sec in m['sections']:
    for k in ('name', 'startBar', 'bars', 'energy', 'description', 'events'):
        if k not in sec:
            err(f'music section missing {k}')
    if sec['startBar'] != bar:
        err(f'music section {sec["name"]} starts bar {sec["startBar"]}, expected {bar}')
    if not (1 <= sec['energy'] <= 5):
        err(f'music section {sec["name"]} energy {sec["energy"]}')
    bar += sec['bars']
    for e in sec['events']:
        if not (0 <= e['frame'] < TOTAL):
            err(f'music event {e} outside film')
        if e['type'] not in {'downbeat-hit', 'drop', 'stop', 'silence', 'riser-start', 'filter-sweep', 'stinger'}:
            err(f'music event type {e["type"]}')
        if e['type'] == 'silence':
            silences.append((e['frame'], e['frame'] + e.get('durationFrames', 15)))
        downbeat0 |= e['frame'] == 0 and e['type'] in ('downbeat-hit', 'drop')
if (bar - 1) * BAR != TOTAL:
    err(f'music sections cover {(bar - 1) * BAR} frames, film is {TOTAL}')
if not downbeat0:
    err('no music hit on frame 0')
if not silences:
    err('no pre-reveal silence in the music map')
for a, b in silences:
    if b - a < 15:
        err(f'silence {a}–{b} shorter than 15 f')
    for st, en, ref, sid in spans:
        if st < b and a < en:
            err(f'{sid}: {ref} sounds inside the silence {a}–{b - 1} (plays {st}–{en - 1})')

# ------------------------------------------------------------------ 12. pacing
shot_starts = sorted(shot_starts)
lens = [b - a for a, b in zip(shot_starts, shot_starts[1:] + [TOTAL])]
end_start = scenes[-1]['startFrame']
for st, ln in zip(shot_starts, lens):
    lim = 180 if st >= end_start else 120
    if ln < 4 or ln > lim:
        err(f'shot at abs {st} lasts {ln} f (allowed 4–{lim})')
if not (28 <= len(shot_starts) <= 38):
    warn(f'{len(shot_starts)} shots (28–38 recommended)')

# ------------------------------------------------------------------ 13. end card
last = scenes[-1]
if last['act'] != 'cta':
    err('last scene is not the CTA end card')
if not (end_tagline and end_cta and end_platforms):
    err('end card must carry the tagline, “Baixe em ubiqx.com.br” and “macOS · Windows · Linux”')
if last['copy']:
    last_land = max(c.get('landFrame', c['inFrame']) for c in last['copy'])
    if last['durationInFrames'] - last_land < 90:
        err(f'end card fully legible for only {last["durationInFrames"] - last_land} f (< 90)')

# ------------------------------------------------------------------ report
sfx_n = len(spans)
print(f'{path}')
print(f'  {len(scenes)} scenes, {TOTAL} frames = {secs:.1f} s ({TOTAL // BAR} bars), {len(shot_starts)} shots '
      f'(avg {TOTAL / max(1, len(shot_starts)) / FPS:.2f} s), transitions {sorted(types)}, SFX cues {sfx_n}, '
      f'max SFX overlap {worst[0]}')
if not QUIET:
    for w in warns:
        print('  WARN', w)
for e in errors:
    print('  ERROR', e)
print('  VALID (0 errors' + (f', {len(warns)} warnings)' if warns else ')') if not errors else f'  {len(errors)} error(s)')
sys.exit(1 if errors else 0)
