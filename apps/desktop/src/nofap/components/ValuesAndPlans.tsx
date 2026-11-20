import { X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Field, Input, Textarea } from '../../components/ui/Field';
import { useT } from '../../i18n';
import { MAX_PLANS, PLAN_TEXT_MAX_CHARS, VALUES_MAX_CHARS } from '../labels';
import type { IfThenPlan } from '../types';

interface Props {
  values: string;
  plans: IfThenPlan[];
  onSaveValues: (text: string) => Promise<void>;
  onSavePlans: (plans: IfThenPlan[]) => Promise<void>;
}

/**
 * The user's own sentence and up to three "se… então…" plans. They are shown back on the support cards, in the
 * panic sheet and while a change is waiting.
 */
export function ValuesAndPlans({ values, plans, onSaveValues, onSavePlans }: Props) {
  const t = useT();
  const [text, setText] = useState(values);
  const [when, setWhen] = useState('');
  const [then, setThen] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => setText(values), [values]);

  const run = async (act: () => Promise<void>) => {
    setBusy(true);
    try {
      await act();
    } finally {
      setBusy(false);
    }
  };

  const addPlan = () =>
    run(async () => {
      await onSavePlans([...plans, { when: when.trim(), then: then.trim() }]);
      setWhen('');
      setThen('');
    });

  return (
    <Card data-testid="values-and-plans">
      <CardHeader title={t('nofap.values.title')} subtitle={t('nofap.checkin.values')} />
      <Field label={t('nofap.values.label')} hint={t('nofap.values.count', { n: [...text].length, max: VALUES_MAX_CHARS })}>
        {(id) => <Textarea id={id} value={text} maxLength={VALUES_MAX_CHARS} placeholder={t('nofap.values.placeholder')} onChange={(e) => setText(e.target.value)} />}
      </Field>
      <div className="mt-2 flex justify-end">
        <Button size="sm" loading={busy} disabled={text.trim() === values} onClick={() => void run(() => onSaveValues(text.trim()))}>
          {t('nofap.btn.save')}
        </Button>
      </div>

      <div className="mt-4 border-t border-line pt-4">
        <h3 className="text-[13px] font-medium text-ink">{t('nofap.plans.label')}</h3>
        <p className="mt-0.5 text-xs text-ink-3">{t('nofap.checkin.ifthen')}</p>
        {plans.length ? (
          <ul className="mt-2 flex flex-col divide-y divide-line">
            {plans.map((plan, i) => (
              <li key={`${plan.when}-${plan.then}-${i}`} className="flex items-start justify-between gap-3 py-2">
                <span className="min-w-0 text-[13px] leading-5 text-ink">{t('nofap.plans.item', { when: plan.when, then: plan.then })}</span>
                <IconButton label={t('nofap.plans.remove', { n: i + 1 })} size="sm" onClick={() => void run(() => onSavePlans(plans.filter((_, j) => j !== i)))}>
                  <X className="size-4" strokeWidth={1.75} />
                </IconButton>
              </li>
            ))}
          </ul>
        ) : (
          <p className="mt-2 text-xs text-ink-3">{t('nofap.plans.empty')}</p>
        )}
        {plans.length < MAX_PLANS ? (
          <div className="mt-3 grid grid-cols-1 gap-3 min-[720px]:grid-cols-2">
            <Field label={t('nofap.plans.when')}>{(id) => <Input id={id} value={when} maxLength={PLAN_TEXT_MAX_CHARS} onChange={(e) => setWhen(e.target.value)} />}</Field>
            <Field label={t('nofap.plans.then')}>{(id) => <Input id={id} value={then} maxLength={PLAN_TEXT_MAX_CHARS} onChange={(e) => setThen(e.target.value)} />}</Field>
            <div className="min-[720px]:col-span-2">
              <Button size="sm" loading={busy} disabled={!when.trim() || !then.trim()} onClick={() => void addPlan()}>
                {t('nofap.plans.add')}
              </Button>
            </div>
          </div>
        ) : (
          <p className="mt-3 text-xs text-ink-3">{t('nofap.plans.limit')}</p>
        )}
      </div>
    </Card>
  );
}
