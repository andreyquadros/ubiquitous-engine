import { BellRing } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useT } from '../../i18n';
import type { FocusSettings } from '../../lib/types';
import { Button } from '../ui/Button';
import { Card, CardHeader } from '../ui/Card';
import { Field, Input } from '../ui/Field';
import { Toggle } from '../ui/Toggle';

interface Props {
  focus: FocusSettings;
  onPatch: (patch: Partial<FocusSettings>) => void;
  onTest: () => Promise<unknown>;
}

const clamp = (v: number, min: number, max: number): number => Math.min(max, Math.max(min, Math.round(v)));

/** Text field that keeps a local draft and commits on blur / Enter (`null` when emptied). */
function ShortcutField({ label, value, onCommit, placeholder }: { label: string; value: string | null; onCommit: (v: string | null) => void; placeholder: string }) {
  const [draft, setDraft] = useState(value ?? '');
  useEffect(() => setDraft(value ?? ''), [value]);
  const commit = () => {
    const next = draft.trim() || null;
    if (next !== value) onCommit(next);
  };
  return (
    <Field label={label}>
      {(id) => (
        <Input
          id={id}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') commit();
          }}
          placeholder={placeholder}
          autoComplete="off"
        />
      )}
    </Field>
  );
}

/** Settings.focus: guard on/off, what a session holds, hiding windows, the default length, the cooldown and the two macOS Shortcut names. */
export function FocusOptions({ focus, onPatch, onTest }: Props) {
  const t = useT();
  const [testing, setTesting] = useState(false);
  const [minutes, setMinutes] = useState(String(focus.session_minutes));
  const [cooldown, setCooldown] = useState(String(focus.intervention_cooldown_secs));
  useEffect(() => setMinutes(String(focus.session_minutes)), [focus.session_minutes]);
  useEffect(() => setCooldown(String(focus.intervention_cooldown_secs)), [focus.intervention_cooldown_secs]);

  const commitMinutes = () => {
    const v = clamp(Number(minutes) || focus.session_minutes, 5, 240);
    setMinutes(String(v));
    if (v !== focus.session_minutes) onPatch({ session_minutes: v });
  };
  const commitCooldown = () => {
    const v = clamp(Number(cooldown) || focus.intervention_cooldown_secs, 5, 600);
    setCooldown(String(v));
    if (v !== focus.intervention_cooldown_secs) onPatch({ intervention_cooldown_secs: v });
  };

  const test = async () => {
    setTesting(true);
    try {
      await onTest();
    } finally {
      setTesting(false);
    }
  };

  return (
    <Card data-testid="focus-options">
      <CardHeader title={t('focus.options.title')} subtitle={t('focus.options.subtitle')} />
      <div className="flex flex-col gap-5">
        <Field label={t('focus.options.guard.label')} hint={t('focus.options.guard.hint')} inline>
          {(id) => <Toggle id={id} checked={focus.guard_enabled} onChange={(v) => onPatch({ guard_enabled: v })} />}
        </Field>
        <Field label={t('focus.options.in_session.label')} hint={t('focus.options.in_session.hint')} inline>
          {(id) => <Toggle id={id} checked={focus.block_distraction_in_session} onChange={(v) => onPatch({ block_distraction_in_session: v })} />}
        </Field>
        <Field label={t('focus.options.hide_others.label')} hint={t('focus.options.hide_others.hint')} inline>
          {(id) => <Toggle id={id} checked={focus.hide_others_on_start} onChange={(v) => onPatch({ hide_others_on_start: v })} />}
        </Field>
        <hr className="border-line" />
        {/* items-end: when one label wraps (the English cooldown label does) the two inputs still share a baseline */}
        <div className="grid grid-cols-1 items-end gap-4 min-[560px]:grid-cols-2">
          <Field label={t('focus.options.session_minutes')}>
            {(id) => (
              <div className="flex items-center gap-2">
                <Input id={id} type="number" min={5} max={240} value={minutes} onChange={(e) => setMinutes(e.target.value)} onBlur={commitMinutes} className="num w-24" />
                <span className="text-xs text-ink-3">{t('focus.options.minutes_suffix')}</span>
              </div>
            )}
          </Field>
          <Field label={t('focus.options.cooldown')}>
            {(id) => (
              <div className="flex items-center gap-2">
                <Input id={id} type="number" min={5} max={600} value={cooldown} onChange={(e) => setCooldown(e.target.value)} onBlur={commitCooldown} className="num w-24" />
                <span className="text-xs text-ink-3">{t('focus.options.cooldown_suffix')}</span>
              </div>
            )}
          </Field>
        </div>
        <hr className="border-line" />
        <div className="flex flex-col gap-4">
          <ShortcutField label={t('focus.options.shortcut_on')} value={focus.macos_focus_shortcut_on} onCommit={(v) => onPatch({ macos_focus_shortcut_on: v })} placeholder={t('focus.options.shortcut_placeholder')} />
          <ShortcutField label={t('focus.options.shortcut_off')} value={focus.macos_focus_shortcut_off} onCommit={(v) => onPatch({ macos_focus_shortcut_off: v })} placeholder={t('focus.options.shortcut_placeholder')} />
          <p className="text-xs leading-5 text-ink-3">{t('focus.options.shortcut_hint')}</p>
        </div>
        <div className="border-t border-line pt-4">
          <Button size="sm" icon={<BellRing className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void test()} loading={testing}>
            {t('focus.options.test')}
          </Button>
        </div>
      </div>
    </Card>
  );
}
