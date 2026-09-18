# ubiqX — design contract

Concept: **instrument panel at night**. A dark, precise, calm cockpit for one person's attention. One thing glows
(the Focus Dial), everything else is quiet hairlines and numbers. The identity comes from UBI: his crest blue (volt)
and his sash orange (ember). Light theme is a daylight version of the same instrument, not an inversion.

Tokens live in `src/index.css`; use Tailwind utilities (`bg-panel`, `text-ink-2`, `text-volt`, `rounded-card`) or the
CSS variables (`var(--volt)`) — never raw hex in components, except category colours, which come from the data.

## 1. Colour

| token | role | dark | light |
|---|---|---|---|
| `canvas` | app background (abyss) | `#060A14` | `#F3F5FA` |
| `panel` | cards, rail | `#0C1220` | `#FFFFFF` |
| `panel-2` | nested surfaces, inputs, hover | `#121A2B` | `#F6F8FC` |
| `panel-3` | tracks, skeletons, pressed | `#18213A` | `#ECEFF6` |
| `line` | hairline border | `rgba(126,158,214,.14)` | `rgba(15,23,42,.08)` |
| `line-2` | stronger hairline (inputs, kbd, glass edge) | `rgba(126,158,214,.28)` | `rgba(15,23,42,.16)` |
| `ink` | primary text | `#EAF0FA` | `#0F172A` |
| `ink-2` | secondary text | `#9DAEC7` | `#475569` |
| `ink-3` | tertiary text (≥ 4.5:1 on panel) | `#7487A6` | `#5B6B85` |
| `ink-4` | non-text marks only: ticks, placeholders, disabled | `#5F6F8C` | `#8B98B0` |
| `volt` | primary accent, focus, links, active nav | `#4D8DFF` | `#2A62D6` |
| `on-volt` | text on a volt fill | `#06101F` | `#FFFFFF` |
| `volt-soft` | volt tint for chips/active rows | 14 % volt | 10 % volt |
| `ember` | attention: review badge, warnings that matter | `#FF7A1F` | `#C2410C` |
| `signal` | productive / focus time | `#2EE6A6` | `#0E9F6E` |
| `rose` | distraction, danger | `#FF5C7A` | `#E11D48` |
| `violet` | meetings, pauses, memory source | `#9B8CFF` | `#6D5BD0` |
| `amber` | caution, paused | `#FFC24D` | `#A3660F` |

Rules: semantic colours (`signal`, `rose`, `violet`, `amber`) are for values, fills and dots; on light they only meet
4.5:1 as **large** text (≥ 18 px semibold), so small copy stays `ink`/`ink-2`. `ember` is rationed: the review count,
the "precisa de revisão" call to action, and warnings the user must act on. Category colours keep coming from the
categories themselves. Legacy `brand-*`/`accent-*`/`surface*`/`line-strong` utilities still resolve (brand = volt,
accent = ember, surface = panel) so unmigrated pages render; new code uses the names above.

## 2. Typography

Sora (display) + Inter (text), both variable, self-hosted (`@fontsource-variable/*`, imported in `main.tsx`).

| role | face | size / line | weight | tracking |
|---|---|---|---|---|
| dial number | Sora | 56 / 1 | 700 | -0.03em |
| page title | Sora | 26 / 32 | 600 | -0.02em |
| big stat | Sora | 22 / 28 | 600 | -0.02em |
| card title | Inter | 14 / 20 | 600 | 0 |
| body | Inter | 14 / 20 or 13 / 18 | 400 | 0 |
| meta / hint | Inter | 12 / 16 | 400–500 | 0 |
| micro (legend, ticks) | Inter | 11 / 14 | 500 | 0 |

Helpers: `.display` (Sora 600, -0.02em), `.num` (tabular figures — every number), `.eyebrow` (12 px ink-3, sentence
case). **No ALL-CAPS labels, no middle-dot meta strings ("A · B"), no "WORD — fragment" labels, no arrows glued to
button text.** Numbers in sentences use pt-BR forms from `lib/format.ts` (`2h10`, `45min`, `88`).

## 3. Space, radius, elevation

- Spacing: 4-pt scale. Page gutter 24 px, gap between cards 20 px, card padding 20 px, dense rows 12 px.
- Content max-width 1280 px, left rail 232 px (72 px below 1180 px wide, icon-only).
- Radius hierarchy: shell `rounded-shell` 20 px (rail, hero, dialogs) → cards `rounded-card` 14 px → controls
  `rounded-control` 10 px (buttons, inputs, tabs) → pills `rounded-pill`. Never the same radius on everything.
- Elevation: hairline borders carry structure; **no shadows on cards.** `shadow-float` only on floating layers
  (`.glass`: popover, dialog, toast, chart tooltip). Glow (`.glow-volt`) only on the Focus Dial, the active nav item
  and primary buttons. `.glow-ember` only on the review CTA.
- Surfaces: `.panel` (card), `.panel-raised` (nested, panel-2 with a 1 px inner highlight), `.glass` (floating).

## 4. Motion

One orchestrated page-load moment, on Hoje only: dial arc draws in 900 ms `ease-out-expo`, the number counts up, the
24h track segments and the hourly bars rise with a 35 ms stagger. UBI floats (y 0 → -8 → 0, 4.2 s, ease-in-out).
Everything else answers a user action: expand/collapse 180 ms, popover 120 ms, dialog 180 ms in / 120 ms out,
toggle 150 ms, hover colour 120 ms. No hover-lift on cards, no fade-up on every section. `useReducedMotion()` from
framer-motion guards every animated component; CSS `prefers-reduced-motion` zeroes durations globally. Animate
transform/opacity/stroke-dashoffset only.

## 5. Charts (recharts)

Grid `var(--chart-grid)` (6 % white in dark), horizontal lines only, no axis lines, ticks `ink-3` 11 px tabular.
Bars rounded 6 px top, category gap 3–4 px. Tooltips are `.glass` with `px-3 py-2 text-xs`. Series colours: focus
→ `signal`, distraction → `rose`, meetings/pauses → `violet`, trend line → `volt`, categories → their own colour.
The 24h track (`DayBar`) is a 34 px rail of coloured segments with a soft glow on hover and a volt now-marker.

## 6. Icons and copy

lucide-react, 18 px in nav/buttons, 14 px inside chips, stroke 1.75 (`strokeWidth={1.75}`). Decorative icons are
`aria-hidden`; icon-only buttons need `label`. Copy is pt-BR, sentence case, verbs first ("Salvar alterações",
"Gerar relatório"), plain words. Errors say what to do next; empty states invite the next action; a headline is a
sentence written from the data, never a label.

## 7. Layout

Hoje (the hero band is the only place with a radial volt glow behind it):

```
┌ rail 232 ┬──────────────────────────────────────────── content ≤ 1280 ───┐
│ ◉ ubiqX  │ Hoje                                        ‹ qui 17/09/2026 › │
│ ▌Hoje    │ ┌─ hero (rounded-shell, hairline, radial volt glow) ─────────┐ │
│  Timeline│ │ (  88  )  2h10 de foco desde o almoço; 8 blocos      [UBI] │ │
│  Revisão⑧│ │  de foco  esperam sua revisão.  ● Precisa de revisão ›     │ │
│  …       │ │           produtivo 8h29  distração 22min  maior foco 1h44 │ │
│          │ └────────────────────────────────────────────────────────────┘ │
│          │ ┌─ Linha do tempo (24h track, now-marker) ───────────────────┐ │
│ ◔ UBI    │ ├─ Tempo por categoria ─┬─ Foco por hora ─┬─ Apps mais usados┤ │
│ ● Rastr. │ └───────────────────────┴─────────────────┴──────────────────┘ │
└──────────┴────────────────────────────────────────────────────────────────┘
```

Standard page:

```
│ Title (Sora 26)                       [actions: DayNav / Button primary] │
│ subtitle in ink-2, one sentence                                          │
│ ┌ panel ───────────────────────────────┐ ┌ panel (aside, 4/12) ────────┐ │
│ │ CardHeader  title ─────────── action │ │                              │ │
│ │ rows divided by hairlines            │ │                              │ │
│ └──────────────────────────────────────┘ └──────────────────────────────┘ │
```

Alignment: left-aligned text everywhere; numbers right-aligned in columns; the dial number centred inside the ring.

## 8. Component inventory (`src/components`)

Layout: `AppShell` (rail + scroll area + Toaster), `Sidebar({ needsReview })`, `TrackerPill()` (state dot, label, mini UBI, pause/resume).

`ui/Card`: `Card({ padded?, tone?: 'panel'|'raised'|'plain', ...div })`, `CardHeader({ title, subtitle?, action?, className? })`,
`StatTile({ label, value, hint?, accent?, icon?, className? })` — label sentence case, value Sora.
`ui/Button`: `Button({ variant?: 'primary'|'secondary'|'ghost'|'danger'|'accent', size?: 'sm'|'md'|'lg', loading?, icon? })`
(`accent` = ember; primary glows), `IconButton({ label, size?, active? })`.
`ui/Badge`: `Badge({ tone?: 'neutral'|'volt'|'ember'|'signal'|'rose'|'violet'|'amber'|'brand'|'accent'|'success'|'warning'|'danger' })`
(the last five are aliases), `StatusPill({ color, children, pulse? })`.
`ui/Field`: `Field({ label, hint?, inline?, children: (id) => node })`, `Input`, `Select`, `Textarea` (native props).
`ui/Toggle({ checked, onChange, id?, disabled?, label?, size? })`. `ui/Dialog({ open, onClose, title, description?, footer?, width? })`.
`ui/Popover({ open, onClose, anchor, align?, className? })`. `ui/TagInput({ value, onChange, placeholder?, id?, disabled? })`.
`ui/DayNav({ date, onChange })`. `ui/PageHeader({ title, subtitle?, actions? })`.
`ui/CategoryChip({ categories, categoryId, size?, interactive? })`, `CategoryPicker({ categories, value, onPick, numbered?, disabled? })`.
`ui/BlockBits`: `SourceBadge({ source })`, `AiSentBadge({ at })`, `ConfidenceBar({ value, showLabel? })`, `AppAvatar({ name, size? })`, `SuggestionChips({ suggestions, categories, onAccept, accepted })`.
`ui/misc`: `Progress({ value, max?, color?, height? })`, `Spinner`, `EmptyState({ icon?, title, description?, action? })`,
`Tabs({ value, onChange, items })`, `Kbd`, `Skeleton`, `SectionTitle({ children, description?, action? })`.
`lib/toast`: `useToast().success|error|info(title, message?)`, `Toaster` (glass, bottom-right).

Charts: `FocusDial({ score, mood, size?, stroke?, label?, animate? })` (alias `FocusRing`), `CategoryDonut({ totals, categories, height? })`,
`DayBar({ blocks, categories, onSelect?, height?, date?, animate? })` (now-marker when `date` is today),
`HourlyFocus({ hourly, height?, animate? })`, `WeeklyStacked({ days, categories, height? })`, `FocusTrend({ days, height? })`.

UBI: `Ubi({ mood, size?, speaking?, variant?: 'auto'|'svg', className? })` picks PNG → GLB → SVG;
`UbiImage({ mood, size?, crop?: 'full'|'head', parallax?, glowFloor? })` (the user's `/ubi/ubi.png`, background removed
client-side); `UbiSvg({ mood, size? })` fallback; `UbiCard({ data, nudge })` = assistant panel (nudge, AI status, budget).

Class vocabulary for pages: `.panel`, `.panel-raised`, `.glass`, `.display`, `.num`, `.eyebrow`, `.glow-volt`,
`.glow-ember`, `.scroll-thin`, `.md`; utilities `bg-canvas|panel|panel-2|panel-3`, `text-ink|ink-2|ink-3`,
`border-line|line-2`, `text-volt|ember|signal|rose|violet|amber`, `bg-volt-soft|ember-soft`, `rounded-shell|card|control|pill`,
`shadow-float|glow`, `font-display`.

## 9. Review against generic defaults

- **Rejected the SaaS-card kit** (identical rounded cards, one radius, grey shadow under each): cards are hairline
  panels without shadows, with a three-level radius hierarchy; only floating layers cast a shadow.
- **Rejected the "near-black + one acid accent" cliché**: the canvas is a deep blue abyss, not tinted black; there are
  two accents with distinct jobs (volt = focus/primary, ember = attention) both taken from UBI's own art.
- **Rejected ALL-CAPS eyebrows and middle-dot meta strings** (the old stat tiles used both): labels are sentence case,
  meta is written as short sentences or separated by spacing.
- **Rejected the "big number + gradient" hero**: the number lives inside an instrument (the dial) next to a
  data-written sentence and the mascot; no gradient washes as decoration, one radial glow behind the hero only.
- **Rejected scattered motion**: no fade-up per section, no hover-lift per card; one load orchestration on Hoje.
- **Rejected a monospace face for data**: Inter with tabular figures; Sora for the few display numbers.
- **Kept**: lucide icons, recharts, the existing routing and every ipc call.
