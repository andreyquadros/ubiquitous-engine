import { useEffect, useRef, type ReactNode } from 'react';
import clsx from 'clsx';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';

interface Props {
  open: boolean;
  onClose: () => void;
  anchor: ReactNode;
  children: ReactNode;
  align?: 'left' | 'right';
  className?: string;
}

/** Anchored glass popover (absolute below the anchor). Closes on outside click and Escape. */
export function Popover({ open, onClose, anchor, children, align = 'left', className }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotion();
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener('mousedown', onDown);
    window.addEventListener('keydown', onKey, true);
    return () => {
      document.removeEventListener('mousedown', onDown);
      window.removeEventListener('keydown', onKey, true);
    };
  }, [open, onClose]);

  return (
    <div ref={ref} className="relative inline-block">
      {anchor}
      <AnimatePresence>
        {open && (
          <motion.div
            initial={reduce ? { opacity: 0 } : { opacity: 0, y: -4, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, y: -4, scale: 0.98 }}
            transition={{ duration: reduce ? 0 : 0.12 }}
            className={clsx('glass absolute z-40 mt-1.5 min-w-56 p-1.5', align === 'right' ? 'right-0' : 'left-0', className)}
          >
            {children}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
