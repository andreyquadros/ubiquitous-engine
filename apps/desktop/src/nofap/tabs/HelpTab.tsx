import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { useT } from '../../i18n';
import { CrisisCard } from '../components/PanicSheet';
import { RemovalHelp } from '../components/RemovalHelp';
import { aiExitText } from '../labels';
import type { NofapDialog } from '../NofapPage';
import type { NofapHost } from '../NofapWizard';
import type { HelpResource, NofapView } from '../types';

interface Props {
  view: NofapView;
  /** `null` while the list is still on its way. */
  resources: HelpResource[] | null;
  host?: NofapHost;
  onOpen: (dialog: NofapDialog) => void;
  onExport: () => void;
  onTestCard: () => void;
}

const TOPICS = ['off', 'remove', 'limits', 'quit', 'ai'] as const;

/** "Ajuda": how to turn it off and remove it, what it does not do, the user's data, and a person to talk to. */
export function HelpTab({ view, resources, host, onOpen, onExport, onTestCard }: Props) {
  const t = useT();
  const name = view.display_name;
  return (
    <div className="flex flex-col gap-5" data-testid="tab-help">
      <Card>
        <CrisisCard resources={resources} />
        <p className="mt-3 border-t border-line pt-3 text-xs leading-5 text-ink-3">{t('nofap.help.limits')}</p>
      </Card>

      <Card>
        <div className="flex flex-col gap-4">
          {TOPICS.map((topic) => (
            <section key={topic} data-testid={`help-${topic}`}>
              <h2 className="text-sm font-semibold text-ink">{t(`nofap.helptab.${topic}.title`)}</h2>
              {topic === 'remove' ? (
                <div className="mt-1">
                  <RemovalHelp os={view.capabilities.os} />
                </div>
              ) : (
                <p className="mt-1 text-[13px] leading-5 text-ink-2">
                  {topic === 'limits' ? `${t('nofap.wizard.limits')} ` : ''}
                  {topic === 'ai' ? aiExitText(view.product, t) : t(`nofap.helptab.${topic}.body`)}
                </p>
              )}
              {topic === 'ai' && host?.openSettings && (
                <div className="mt-2">
                  <Button size="sm" onClick={host.openSettings}>
                    {t('nofap.btn.open_settings')}
                  </Button>
                </div>
              )}
            </section>
          ))}
        </div>
      </Card>

      <Card data-testid="help-data">
        <h2 className="text-sm font-semibold text-ink">{t('nofap.helptab.data.title')}</h2>
        <p className="mt-1 text-[13px] leading-5 text-ink-2">{t('nofap.helptab.data.body')}</p>
        <p className="mt-2 text-xs leading-5 text-ink-3">{t('nofap.privacy.at_rest', { name })}</p>
        <div className="mt-3 flex flex-wrap gap-2">
          <Button size="sm" onClick={onExport}>
            {t('nofap.data.export')}
          </Button>
          <Button size="sm" onClick={() => onOpen('erase')}>
            {t('nofap.data.erase', { name })}
          </Button>
          <Button size="sm" onClick={onTestCard}>
            {t('nofap.data.test_card')}
          </Button>
        </div>
        <p className="mt-3 text-xs leading-5 text-ink-3">{t('nofap.data.reset_note', { name })}</p>
      </Card>
    </div>
  );
}
