import { X } from 'lucide-react';
import { useState, type FormEvent, type ReactNode } from 'react';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Field, Input } from '../../components/ui/Field';
import { Toggle } from '../../components/ui/Toggle';
import { useT } from '../../i18n';
import { delayVariant, fmtDelay, waitingSwitches } from '../labels';
import { WaitingNote } from './WaitingNote';
import type { NofapChange, NofapView, SiteEntry, SitesView } from '../types';

const looksLikeHost = (text: string): boolean => /^[^\s/]+\.[^\s/.]{2,}$/.test(text.trim().replace(/^[a-z]+:\/\//i, '').replace(/[/?#].*$/, ''));

interface ListProps {
  title: string;
  subtitle: string;
  inputLabel: string;
  addLabel: string;
  empty: string;
  entries: SiteEntry[];
  onAdd: (host: string) => Promise<void>;
  onRemove: (entry: SiteEntry) => void;
  /** Something that belongs to this list, shown under it. */
  footer?: ReactNode;
  testId: string;
}

function SiteList({ title, subtitle, inputLabel, addLabel, empty, entries, onAdd, onRemove, footer, testId }: ListProps) {
  const t = useT();
  const [text, setText] = useState('');
  const [invalid, setInvalid] = useState(false);
  const [busy, setBusy] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!looksLikeHost(text)) {
      setInvalid(true);
      return;
    }
    setBusy(true);
    try {
      await onAdd(text.trim());
      setText('');
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card data-testid={testId}>
      <CardHeader title={title} subtitle={subtitle} />
      <form onSubmit={(e) => void submit(e)} className="flex flex-wrap items-start gap-2">
        <div className="min-w-0 flex-1 basis-56">
          <Input
            aria-label={inputLabel}
            aria-invalid={invalid}
            value={text}
            placeholder={t('nofap.lists.placeholder')}
            autoComplete="off"
            spellCheck={false}
            onChange={(e) => {
              setText(e.target.value);
              setInvalid(false);
            }}
          />
          {invalid && (
            <p role="alert" className="mt-1 text-xs text-ink-2">
              {t('nofap.lists.invalid')}
            </p>
          )}
        </div>
        <Button type="submit" loading={busy} disabled={!text.trim()}>
          {addLabel}
        </Button>
      </form>
      {entries.length ? (
        <ul className="mt-3 flex flex-col divide-y divide-line">
          {entries.map((entry) => (
            <li key={entry.id} className="flex items-center justify-between gap-3 py-2">
              <span className="min-w-0 truncate text-sm text-ink" title={entry.host}>
                {entry.host}
              </span>
              <IconButton label={t('nofap.lists.remove_site', { host: entry.host })} size="sm" onClick={() => onRemove(entry)}>
                <X className="size-4" strokeWidth={1.75} />
              </IconButton>
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-3 text-xs text-ink-3">{empty}</p>
      )}
      {footer && <div className="mt-3 border-t border-line pt-3">{footer}</div>}
    </Card>
  );
}

interface Props {
  view: NofapView;
  sites: SitesView;
  /** Sends the change; the caller tells the user whether it applied or waits. */
  onRequest: (change: NofapChange) => Promise<void>;
}

/**
 * The lists the user typed: sites to stop ("sites barrados", never "bloqueados", which Settings uses for
 * something else) and sites never to stop. Learned sites are a count; their names are shown nowhere.
 */
export function SiteLists({ view, sites, onRequest }: Props) {
  const t = useT();
  const name = view.display_name;
  const delaySecs = view.config.delay_secs;
  const delay = fmtDelay(delaySecs, t);
  // Taking the typed sites out of the system list waits like any loosening; while it does, the switch is disabled.
  const userSitesWaiting = waitingSwitches(view.config).hostsUserSites;
  return (
    <div className="flex flex-col gap-5">
      <SiteList
        testId="sites-blocked"
        title={t('nofap.lists.blocked.title')}
        subtitle={t(delayVariant('nofap.lists.blocked.subtitle', delaySecs), { delay })}
        inputLabel={t('nofap.lists.blocked.input')}
        addLabel={t('nofap.lists.add_blocked')}
        empty={t('nofap.lists.empty_blocked')}
        entries={sites.blocked}
        onAdd={(host) => onRequest({ kind: 'add_block', host })}
        onRemove={(entry) => void onRequest({ kind: 'remove_block', id: entry.id })}
        // Where the sites the user types go: into the system list too, which other accounts can read, or only into this app.
        footer={
          <div className="flex flex-col gap-2">
            <Field label={t('nofap.lists.system_sites')} hint={t('nofap.hosts.readable')} inline>
              {(id) => <Toggle id={id} size="sm" checked={view.config.system.hosts_user_sites} disabled={userSitesWaiting} onChange={(on) => void onRequest({ kind: 'set_hosts_user_sites', on })} />}
            </Field>
            {userSitesWaiting && <WaitingNote testId="waiting-set_hosts_user_sites" />}
          </div>
        }
      />
      <SiteList
        testId="sites-allowed"
        title={t('nofap.lists.allowed.title')}
        subtitle={t(delayVariant('nofap.lists.allowed.subtitle', delaySecs), { name, delay })}
        inputLabel={t('nofap.lists.allowed.input')}
        addLabel={t('nofap.lists.add_allowed')}
        empty={t('nofap.lists.empty_allowed')}
        entries={sites.allowed}
        onAdd={(host) => onRequest({ kind: 'add_allow', host })}
        onRemove={(entry) => void onRequest({ kind: 'remove_allow', id: entry.id })}
      />
      <Card data-testid="sites-learned">
        <CardHeader
          title={t('nofap.lists.learned.title')}
          subtitle={sites.learned_count > 0 ? t('nofap.lists.learned.count', { count: sites.learned_count }) : t('nofap.lists.learned.none')}
          action={
            sites.learned_count > 0 ? (
              <Button size="sm" onClick={() => void onRequest({ kind: 'forget_learned' })}>
                {t('nofap.lists.learned.forget')}
              </Button>
            ) : undefined
          }
        />
      </Card>
    </div>
  );
}
