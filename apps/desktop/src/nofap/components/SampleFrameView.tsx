import { useState } from 'react';
import { Button } from '../../components/ui/Button';
import { useT } from '../../i18n';
import { nofapIpc } from '../ipc';
import type { SampleFrame } from '../types';

/**
 * "Ver um exemplo do que é enviado": the reduced image of the last candidate window, fetched only when the user
 * asks. The example itself is never sent and never stored.
 */
export function SampleFrameView() {
  const t = useT();
  const [frame, setFrame] = useState<SampleFrame | null | undefined>(undefined);
  const [busy, setBusy] = useState(false);

  const load = async () => {
    setBusy(true);
    try {
      setFrame(await nofapIpc.sampleFrame());
    } catch {
      setFrame(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-2" data-testid="sample-frame">
      <div>
        <Button size="sm" loading={busy} onClick={() => void load()}>
          {t('nofap.vision.sample')}
        </Button>
      </div>
      {frame === null && <p className="text-xs text-ink-3">{t('nofap.vision.sample_none')}</p>}
      {frame && (
        <figure className="flex flex-col gap-1.5">
          <img src={`data:${frame.mime};base64,${frame.base64}`} alt={t('nofap.vision.sample_alt')} width={frame.width} height={frame.height} className="h-auto w-full max-w-[320px] rounded-control border border-line-2" />
          <figcaption className="text-xs text-ink-3">{t('nofap.vision.sample_caption', { width: frame.width, height: frame.height })}</figcaption>
        </figure>
      )}
    </div>
  );
}
