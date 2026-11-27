import { Field } from '../../components/ui/Field';
import { Toggle } from '../../components/ui/Toggle';
import { useT } from '../../i18n';
import { SCOPE_FLAGS, type ScopeFlag } from '../labels';
import type { VisionScope as Scope } from '../types';
import { WaitingNote } from './WaitingNote';

interface Props {
  scope: Scope;
  onChange: (scope: Scope) => void;
  disabled?: boolean;
  /** The switches whose turning off is waiting out the delay: disabled, with a line that says so. */
  waiting?: Record<ScopeFlag, boolean>;
}

/** Which windows the AI check may look at. */
export function VisionScope({ scope, onChange, disabled, waiting }: Props) {
  const t = useT();
  return (
    <div className="flex flex-col gap-3" role="group" aria-label={t('nofap.vision.scope_title')}>
      {SCOPE_FLAGS.map((flag) => (
        <Field key={flag} label={t(`nofap.scope.${flag}`)} hint={t(`nofap.scope.${flag}.hint`)} inline>
          {(id) => <Toggle id={id} size="sm" checked={scope[flag]} disabled={disabled || waiting?.[flag]} onChange={(v) => onChange({ ...scope, [flag]: v })} />}
        </Field>
      ))}
      {waiting && SCOPE_FLAGS.some((flag) => waiting[flag]) && <WaitingNote text="nofap.pending.scope_waiting" testId="waiting-set_vision_scope" />}
    </div>
  );
}
