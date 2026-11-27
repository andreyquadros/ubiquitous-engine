import clsx from 'clsx';
import { ChevronDown, ImageOff } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useT } from '../../i18n';
import { fmtDuration, fmtTime } from '../../lib/format';
import { ipc } from '../../lib/ipc';
import type { ActivityBlock, Id, ScreenshotData } from '../../lib/types';
import { AiSentBadge, ConfidenceBar, SourceBadge } from './BlockBits';
import { Dialog } from './Dialog';
import { Skeleton } from './misc';

/** Screenshots are fetched once per block and kept for the session; a deleted capture stays `null`. */
const cache = new Map<Id, Promise<ScreenshotData | null>>();

/** Test hook: forget every fetched screenshot. */
export const __clearScreenshotCache = (): void => cache.clear();

export function loadScreenshot(blockId: Id): Promise<ScreenshotData | null> {
  let p = cache.get(blockId);
  if (!p) {
    p = ipc.getScreenshot(blockId).catch((e: unknown) => {
      cache.delete(blockId);
      throw e;
    });
    cache.set(blockId, p);
  }
  return p;
}

export const screenshotUrl = (shot: ScreenshotData): string => `data:${shot.mime};base64,${shot.data_base64}`;

const secsBetween = (a: string, b: string): number => (new Date(b).getTime() - new Date(a).getTime()) / 1000;

/** Human range of a block: "08:15 até 08:19, 4min". */
export function blockRange(t: ReturnType<typeof useT>, b: Pick<ActivityBlock, 'started_at' | 'ended_at'>): string {
  return t('review.block.range', { from: fmtTime(b.started_at), to: fmtTime(b.ended_at), duration: fmtDuration(secsBetween(b.started_at, b.ended_at), { compact: true }) });
}

/** JSON payloads read better indented; anything else is shown as sent. */
const prettyPayload = (raw: string): string => {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
};

type ThumbState = { status: 'loading' } | { status: 'ready'; shot: ScreenshotData } | { status: 'gone' } | { status: 'error' };

/** Thumbnail of the block's stored screenshot; click opens it larger in a Dialog. Fetched lazily on mount. */
export function ScreenshotThumb({ block }: { block: ActivityBlock }) {
  const t = useT();
  const [state, setState] = useState<ThumbState>({ status: 'loading' });
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let alive = true;
    setState({ status: 'loading' });
    loadScreenshot(block.id).then(
      (shot) => alive && setState(shot ? { status: 'ready', shot } : { status: 'gone' }),
      () => alive && setState({ status: 'error' }),
    );
    return () => {
      alive = false;
    };
  }, [block.id]);

  if (state.status === 'loading') {
    return (
      <div className="w-40 shrink-0 self-start" role="status" aria-label={t('review.block.screenshot_loading')} data-testid="screenshot-skeleton">
        <Skeleton className="aspect-[8/5] w-full" />
      </div>
    );
  }
  if (state.status !== 'ready') {
    return (
      <div className="flex w-40 shrink-0 flex-col gap-1 self-start text-xs leading-4 text-ink-3">
        <span className="flex items-center gap-1.5 text-ink-2">
          <ImageOff className="size-3.5 shrink-0" strokeWidth={1.75} aria-hidden />
          {t('review.block.screenshot_gone')}
        </span>
        <span>{t('review.block.keep_hint')}</span>
      </div>
    );
  }
  const alt = t('review.block.screenshot_alt', { app: block.app_name, time: fmtTime(block.started_at) });
  const src = screenshotUrl(state.shot);
  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        title={t('review.block.screenshot_open')}
        aria-label={t('review.block.screenshot_open')}
        className="group/shot w-40 shrink-0 self-start overflow-hidden rounded-control border border-line bg-panel-3 transition-[border-color] duration-150 hover:border-line-2 focus-visible:outline-2 focus-visible:outline-volt"
      >
        <img src={src} alt={alt} width={state.shot.width ?? undefined} height={state.shot.height ?? undefined} className="block aspect-[8/5] w-full object-cover transition-transform duration-180 group-hover/shot:scale-[1.02]" loading="lazy" />
      </button>
      <Dialog open={open} onClose={() => setOpen(false)} title={block.title || block.app_name} description={t('review.block.screenshot_dialog', { app: block.app_name, range: blockRange(t, block) })} width="lg">
        <img src={src} alt={alt} className="mx-auto block max-h-[56vh] w-auto max-w-full rounded-control border border-line" />
      </Dialog>
    </>
  );
}

/** The exact text that went to the classifier, collapsed by default. */
function AiPayload({ payload }: { payload: string }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <div>
      <button type="button" aria-expanded={open} onClick={() => setOpen((o) => !o)} className="inline-flex h-6 items-center gap-1 rounded-md text-[11px] font-medium text-ink-2 transition-colors duration-150 hover:text-ink">
        <ChevronDown className={clsx('size-3.5 transition-transform duration-180', open && 'rotate-180')} strokeWidth={1.75} aria-hidden />
        {open ? t('review.block.ai_payload_hide') : t('review.block.ai_payload')}
      </button>
      {open && <pre className="scroll-thin mt-1 max-h-48 overflow-auto rounded-control border border-line bg-panel px-3 py-2 font-mono text-[11px] leading-4 whitespace-pre-wrap text-ink-2">{prettyPayload(payload)}</pre>}
    </div>
  );
}

/** One block of a review group with everything a human needs to decide its category. */
export function BlockCard({ block }: { block: ActivityBlock }) {
  const t = useT();
  const link = block.url ?? block.domain;
  return (
    <article className="panel-raised flex flex-col gap-3 p-3 min-[900px]:flex-row" data-testid="block-card" aria-label={block.title || block.app_name}>
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div className="min-w-0">
          <p className="num text-xs leading-4 text-ink-3">{blockRange(t, block)}</p>
          <p className="truncate text-sm leading-5 font-medium text-ink" title={block.title || undefined}>
            {block.title || t('review.block.no_title')}
          </p>
          {link && (
            <p className="truncate text-xs leading-4 text-ink-3" title={link}>
              {link}
            </p>
          )}
        </div>
        {block.description && <p className="text-xs leading-4 text-ink-2">{block.description}</p>}
        <div className="flex flex-wrap items-center gap-2">
          <SourceBadge source={block.source} />
          <ConfidenceBar value={block.confidence} />
          <AiSentBadge at={block.ai_sent_at} />
        </div>
        {block.ai_payload ? <AiPayload payload={block.ai_payload} /> : <p className="text-[11px] leading-4 text-ink-3">{t('review.block.not_sent')}</p>}
      </div>
      {block.screenshot_id ? (
        <ScreenshotThumb block={block} />
      ) : (
        <div className="flex w-40 shrink-0 flex-col gap-1 self-start text-xs leading-4 text-ink-3">
          <span className="flex items-center gap-1.5 text-ink-2">
            <ImageOff className="size-3.5 shrink-0" strokeWidth={1.75} aria-hidden />
            {t('review.block.no_screenshot')}
          </span>
          <span>{t('review.block.keep_hint')}</span>
        </div>
      )}
    </article>
  );
}

/** Every block of a group, oldest first, with a skeleton while the day's timeline is still loading. */
export function BlockDetails({ blocks, loading, className }: { blocks: ActivityBlock[]; loading?: boolean; className?: string }) {
  const t = useT();
  if (loading && !blocks.length) {
    return (
      <div className={clsx('flex flex-col gap-2', className)} aria-busy="true" aria-label={t('review.details.loading')}>
        <Skeleton className="h-20" />
        <Skeleton className="h-20" />
      </div>
    );
  }
  const sorted = [...blocks].sort((a, b) => a.started_at.localeCompare(b.started_at));
  return (
    <div className={clsx('flex flex-col gap-2', className)} data-testid="block-details">
      <p className="text-[11px] leading-4 text-ink-3">{t('review.details.blocks', { count: sorted.length })}</p>
      {sorted.map((b) => (
        <BlockCard key={b.id} block={b} />
      ))}
    </div>
  );
}
