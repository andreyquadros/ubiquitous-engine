import { Apple, Check, Copy, Download, ExternalLink, Monitor, Terminal } from 'lucide-react';
import { useState } from 'react';
import { Button, ButtonLink } from '../components/Button';
import { SectionHeader } from '../components/SectionHeader';
import { DOWNLOAD_URLS, MACOS_FIX_COMMANDS, MACOS_GUIDE_URL, RELEASES_URL } from '../config';
import { useCopy } from '../i18n';

const ICONS = { macos: Apple, windows: Monitor, linux: Terminal } as const;

export function Downloads() {
  const { copy } = useCopy();
  const s = copy.downloads;
  const [copied, setCopied] = useState(false);
  const commands = MACOS_FIX_COMMANDS.join('\n');

  const copyCommands = async () => {
    try {
      await navigator.clipboard.writeText(commands);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      /* the block is selectable anyway */
    }
  };

  return (
    <section id="baixar" className="container-site py-16 sm:py-24" aria-labelledby="downloads-title">
      <SectionHeader id="downloads-title" eyebrow={s.eyebrow} title={s.title} intro={s.intro} />
      <ul className="mt-10 grid gap-4 sm:grid-cols-3">
        {s.os.map((os) => {
          const Icon = ICONS[os.id];
          const url = DOWNLOAD_URLS[os.id];
          return (
            <li key={os.id} className="panel flex flex-col p-5">
              <Icon size={22} strokeWidth={1.75} className="text-ink-2" aria-hidden="true" />
              <h3 className="mt-4 text-base font-semibold">{os.name}</h3>
              <p className="mt-1 text-sm text-ink-2">{os.requires}</p>
              <div className="mt-5">
                {url ? (
                  <ButtonLink href={url} className="w-full" icon={<Download size={16} strokeWidth={1.75} aria-hidden="true" />}>
                    {os.cta}
                  </ButtonLink>
                ) : (
                  <Button className="w-full" variant="secondary" disabled>
                    {s.soon}
                  </Button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
      <p className="mt-4">
        <a href={RELEASES_URL} className="inline-flex items-center gap-1.5 text-sm text-volt underline-offset-4 hover:underline" rel="noopener">
          {s.releases}
          <ExternalLink size={14} strokeWidth={1.75} aria-hidden="true" />
        </a>
      </p>

      <div className="panel-raised mt-10 grid gap-6 p-5 sm:p-7 lg:grid-cols-2">
        <div className="min-w-0">
          <h3 className="text-base font-semibold">{s.macos.title}</h3>
          <ol className="mt-3 flex list-decimal flex-col gap-2 pl-5 text-sm leading-6 text-ink-2 marker:text-ink-3">
            {s.macos.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
          <p className="mt-4">
            <a href={MACOS_GUIDE_URL} className="inline-flex items-center gap-1.5 text-sm text-volt underline-offset-4 hover:underline" rel="noopener">
              {s.macos.guide}
              <ExternalLink size={14} strokeWidth={1.75} aria-hidden="true" />
            </a>
          </p>
        </div>
        <div className="min-w-0">
          <p className="text-sm leading-6 text-ink-2">{s.macos.damaged}</p>
          <div className="mt-3">
            {/* Wrapped, never overlaid: both commands stay fully readable at every width without horizontal scroll. */}
            <pre className="rounded-control border border-line bg-canvas p-4 font-mono text-[12px] leading-6 whitespace-pre-wrap break-all text-ink">
              <code>{commands}</code>
            </pre>
            <div className="mt-2 flex sm:justify-end">
              <Button variant="secondary" className="w-full sm:w-auto" onClick={copyCommands} icon={copied ? <Check size={14} strokeWidth={2} className="text-signal" aria-hidden="true" /> : <Copy size={14} strokeWidth={1.75} aria-hidden="true" />}>
                {copied ? s.macos.copied : s.macos.copy}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
