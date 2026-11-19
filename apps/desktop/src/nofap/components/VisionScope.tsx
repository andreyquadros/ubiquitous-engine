import { Field } from '../../components/ui/Field';
import { Toggle } from '../../components/ui/Toggle';
import { useT } from '../../i18n';
import type { VisionScope as Scope } from '../types';

const FLAGS = ['unknown_sites', 'media_apps', 'messaging_apps', 'private_windows'] as const;

/** Which windows the AI check may look at. */
export function VisionScope({ scope, onChange, disabled }: { scope: Scope; onChange: (scope: Scope) => void; disabled?: boolean }) {
  const t = useT();
  return (
    <div className="flex flex-col gap-3" role="group" aria-label={t('nofap.vision.scope_title')}>
      {FLAGS.map((flag) => (
        <Field key={flag} label={t(`nofap.scope.${flag}`)} hint={t(`nofap.scope.${flag}.hint`)} inline>
          {(id) => <Toggle id={id} size="sm" checked={scope[flag]} disabled={disabled} onChange={(v) => onChange({ ...scope, [flag]: v })} />}
        </Field>
      ))}
    </div>
  );
}
