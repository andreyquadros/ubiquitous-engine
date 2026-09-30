import { Play, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useCopy } from '../i18n';
import { cx } from './cx';

const POSTER = `${import.meta.env.BASE_URL}video/ubiqx-trailer-poster.jpg`;
const SRC = `${import.meta.env.BASE_URL}video/ubiqx-trailer-1080p.mp4`;

/**
 * A second bubble beside the mascot, this one clickable: it opens the launch trailer in a modal.
 * The file is only fetched when the dialog opens (`preload="none"`), so the hero stays light.
 */
export function VideoBubble({ className }: { className?: string }) {
  const { copy } = useCopy();
  const v = copy.hero.video;
  const [open, setOpen] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const video = useRef<HTMLVideoElement>(null);
  const opener = useRef<HTMLButtonElement>(null);

  const close = useCallback(() => {
    setOpen(false);
    // the browser hands the focus back to the body when the modal closes, so claim it afterwards
    setTimeout(() => opener.current?.focus(), 0);
  }, []);

  useEffect(() => {
    const d = dialog.current;
    if (!d) return;
    if (open && !d.open) {
      d.showModal();
      video.current?.play().catch(() => {
        /* autoplay refused: the controls are there */
      });
    }
    if (!open && d.open) {
      video.current?.pause();
      d.close();
    }
  }, [open]);

  return (
    <>
      <button
        ref={opener}
        type="button"
        onClick={() => setOpen(true)}
        className={cx(
          'group relative z-20 flex items-center gap-2.5 rounded-2xl border border-line-2 bg-panel-2/95 px-3 py-2 text-left shadow-lg backdrop-blur transition-colors duration-150 hover:border-volt/70 hover:bg-panel-3',
          className,
        )}
      >
        <span className="relative grid h-9 w-9 shrink-0 place-items-center rounded-full bg-volt text-on-volt">
          <span className="ubi-ping absolute inset-0 rounded-full bg-volt" aria-hidden="true" />
          <Play size={15} strokeWidth={2} fill="currentColor" className="relative ml-0.5" aria-hidden="true" />
        </span>
        <span className="min-w-0">
          <span className="block text-[12.5px] font-semibold leading-snug text-ink sm:text-[13px]">{v.bubble}</span>
          <span className="block text-[11px] leading-snug text-ink-3">{v.hint}</span>
        </span>
        <span className="absolute -top-1.5 left-8 h-3 w-3 rotate-45 border-l border-t border-line-2 bg-panel-2/95 transition-colors group-hover:border-volt/70 group-hover:bg-panel-3" aria-hidden="true" />
      </button>

      <dialog
        ref={dialog}
        onClose={close}
        onClick={(e) => {
          if (e.target === dialog.current) close();
        }}
        // Chrome's own video controls swallow Escape while the player has focus, so close it here too.
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault();
            close();
          }
        }}
        aria-label={v.title}
        className="m-auto w-[min(96vw,1120px)] max-w-none rounded-card border border-line-2 bg-panel p-0 text-ink shadow-float backdrop:bg-canvas/85 backdrop:backdrop-blur-sm"
      >
        <figure className="relative">
          {open && (
            <video
              ref={video}
              className="block aspect-video w-full rounded-t-card bg-black"
              controls
              playsInline
              preload="none"
              poster={POSTER}
              src={SRC}
            >
              {v.unsupported}
            </video>
          )}
          <figcaption className="flex items-center justify-between gap-4 px-4 py-3">
            <span>
              <span className="block text-sm font-semibold">{v.title}</span>
              <span className="block text-[13px] text-ink-2">{v.caption}</span>
            </span>
            <button type="button" className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-control hover:bg-panel-2" aria-label={v.close} onClick={close}>
              <X size={18} strokeWidth={1.75} aria-hidden="true" />
            </button>
          </figcaption>
        </figure>
      </dialog>
    </>
  );
}
