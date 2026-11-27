import { CircleAlert } from 'lucide-react';
import { Button } from '../../components/ui/Button';
import { useT } from '../../i18n';
import type { Issue } from '../types';

/** What the one button of an issue does; most issues are information only. */
export type IssueAction = 'restore_network' | 'apply' | 'consent' | 'restore_defaults';

const ACTIONS: Partial<Record<Issue, IssueAction>> = {
  resolver_unreachable: 'restore_network',
  new_connection_unfiltered: 'apply',
  consent_outdated: 'consent',
  config_unreadable: 'restore_defaults',
};

const ACTION_LABEL: Record<IssueAction, string> = {
  restore_network: 'nofap.btn.restore_network',
  apply: 'nofap.btn.apply_again',
  consent: 'nofap.btn.review_consent',
  restore_defaults: 'nofap.btn.restore_defaults',
};

/** Two issues reuse a sentence of the copy bank; the others have their own. */
const COPY: Partial<Record<Issue, string>> = {
  resolver_unreachable: 'nofap.resolver.unreachable',
  other_product_primary: 'nofap.secondary.note',
};

/** Everything the page tells the user about: one sentence each, and at most one action. */
export function IssueList({ issues, onAction }: { issues: Issue[]; onAction: (action: IssueAction) => void }) {
  const t = useT();
  if (!issues.length) return null;
  return (
    <ul className="flex flex-col gap-2" aria-label={t('nofap.issue.title')} data-testid="issue-list">
      {issues.map((issue) => {
        const action = ACTIONS[issue];
        return (
          <li key={issue} className="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-control border border-amber/30 bg-amber/10 px-3 py-2.5" data-testid={`issue-${issue}`}>
            <CircleAlert className="size-4 shrink-0 text-amber" strokeWidth={1.75} aria-hidden />
            <p className="min-w-0 flex-1 basis-56 text-[13px] leading-5 text-ink">{t(COPY[issue] ?? `nofap.issue.${issue}`)}</p>
            {action && (
              <Button size="sm" onClick={() => onAction(action)}>
                {t(ACTION_LABEL[action])}
              </Button>
            )}
          </li>
        );
      })}
    </ul>
  );
}
