import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { X } from 'lucide-react';
import { useEffect, useRef, type ReactNode } from 'react';
import clsx from 'clsx';
import { IconButton } from './Button';
import { useT } from '../../i18n';

interface Props {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  width?: 'sm' | 'md' | 'lg';
}

/** Modal on a glass panel. Escape and backdrop close it; focus moves to the first control. */
export function Dialog({ open, onClose, title, description, children, footer, width = 'md' }: Props) {
  const panel = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotion();
  const t = useT();
  const closeLabel = t('common.close');
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    const timer = setTimeout(() => {
      const el = panel.current?.querySelector<HTMLElement>(`input, select, textarea, button:not([aria-label="${closeLabel}"])`);
      el?.focus();
    }, 30);
    return () => {
      window.removeEventListener('keydown', onKey);
      clearTimeout(timer);
    };
  }, [open, onClose, closeLabel]);

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="fixed inset-0 z-[90] flex items-center justify-center bg-[#02040a]/60 p-6 backdrop-blur-[3px]"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: reduce ? 0 : 0.15 }}
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) onClose();
          }}
        >
          <motion.div
            ref={panel}
            role="dialog"
            aria-modal="true"
            aria-label={typeof title === 'string' ? title : undefined}
            className={clsx('glass max-h-[85vh] w-full overflow-hidden rounded-shell', width === 'sm' ? 'max-w-sm' : width === 'lg' ? 'max-w-3xl' : 'max-w-lg')}
            initial={reduce ? { opacity: 0 } : { opacity: 0, y: 10, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, y: 6, scale: 0.98 }}
            transition={{ duration: reduce ? 0 : 0.18, ease: [0.16, 1, 0.3, 1] }}
          >
            <div className="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
              <div className="min-w-0">
                <h2 className="display text-base">{title}</h2>
                {description && <p className="mt-0.5 text-xs text-ink-2">{description}</p>}
              </div>
              <IconButton label={closeLabel} onClick={onClose} size="sm">
                <X className="size-4" strokeWidth={1.75} />
              </IconButton>
            </div>
            <div className="scroll-thin max-h-[60vh] overflow-y-auto px-5 py-4">{children}</div>
            {footer && <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">{footer}</div>}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
