import { ChevronLeft, ChevronRight, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { SectionHeader } from '../components/SectionHeader';
import { fill, useCopy } from '../i18n';

const SHOTS = `${import.meta.env.BASE_URL}shots/`;

export function Screens() {
  const { copy } = useCopy();
  const s = copy.screens;
  const [open, setOpen] = useState<number | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const opener = useRef<HTMLElement | null>(null);

  const close = useCallback(() => {
    setOpen(null);
    opener.current?.focus();
  }, []);

  useEffect(() => {
    const d = dialog.current;
    if (!d) return;
    if (open !== null && !d.open) d.showModal();
    if (open === null && d.open) d.close();
  }, [open]);

  useEffect(() => {
    if (open === null) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'ArrowRight') setOpen((i) => (i === null ? i : (i + 1) % s.shots.length));
      if (e.key === 'ArrowLeft') setOpen((i) => (i === null ? i : (i - 1 + s.shots.length) % s.shots.length));
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, s.shots.length]);

  const current = open === null ? null : s.shots[open];

  return (
    <section id="telas" className="container-site py-16 sm:py-24" aria-labelledby="screens-title">
      <SectionHeader id="screens-title" eyebrow={s.eyebrow} title={s.title} intro={s.intro} />
      <ul className="mt-10 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        {s.shots.map((shot, i) => (
          <li key={shot.file} className={i === 0 ? 'sm:col-span-2 lg:col-span-2' : undefined}>
            <figure className="panel overflow-hidden">
              <button
                type="button"
                className="block w-full bg-panel-2"
                aria-label={fill(s.open, { title: shot.title })}
                onClick={(e) => {
                  opener.current = e.currentTarget;
                  setOpen(i);
                }}
              >
                <img
                  src={`${SHOTS}${shot.file}.png`}
                  alt={`${shot.title}: ${shot.caption}`}
                  width={1440}
                  height={900}
                  loading={i < 2 ? 'eager' : 'lazy'}
                  decoding="async"
                  className="aspect-[16/10] w-full object-cover object-top"
                />
              </button>
              <figcaption className="border-t border-line px-4 py-3">
                <span className="block text-sm font-semibold">{shot.title}</span>
                <span className="mt-0.5 block text-[13px] leading-5 text-ink-2">{shot.caption}</span>
              </figcaption>
            </figure>
          </li>
        ))}
      </ul>

      <dialog
        ref={dialog}
        onClose={close}
        onClick={(e) => {
          if (e.target === dialog.current) close();
        }}
        aria-label={current ? current.title : undefined}
        className="m-auto w-[min(96vw,1280px)] max-w-none rounded-card border border-line-2 bg-panel p-0 text-ink shadow-float backdrop:bg-canvas/85 backdrop:backdrop-blur-sm"
      >
        {current && (
          <figure className="relative">
            <img src={`${SHOTS}${current.file}.png`} alt={`${current.title}: ${current.caption}`} width={1440} height={900} className="block w-full rounded-t-card" />
            <figcaption className="flex items-center justify-between gap-4 px-4 py-3">
              <span>
                <span className="block text-sm font-semibold">{current.title}</span>
                <span className="block text-[13px] text-ink-2">{current.caption}</span>
              </span>
              <span className="flex shrink-0 items-center gap-1">
                <button type="button" className="inline-flex h-9 w-9 items-center justify-center rounded-control hover:bg-panel-2" aria-label={s.prev} onClick={() => setOpen((i) => (i === null ? i : (i - 1 + s.shots.length) % s.shots.length))}>
                  <ChevronLeft size={18} strokeWidth={1.75} aria-hidden="true" />
                </button>
                <button type="button" className="inline-flex h-9 w-9 items-center justify-center rounded-control hover:bg-panel-2" aria-label={s.next} onClick={() => setOpen((i) => (i === null ? i : (i + 1) % s.shots.length))}>
                  <ChevronRight size={18} strokeWidth={1.75} aria-hidden="true" />
                </button>
                <button type="button" className="inline-flex h-9 w-9 items-center justify-center rounded-control hover:bg-panel-2" aria-label={s.close} onClick={close}>
                  <X size={18} strokeWidth={1.75} aria-hidden="true" />
                </button>
              </span>
            </figcaption>
          </figure>
        )}
      </dialog>
    </section>
  );
}
