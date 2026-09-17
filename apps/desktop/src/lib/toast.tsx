import { create } from 'zustand';
import { AnimatePresence, motion } from 'framer-motion';
import { CheckCircle2, CircleAlert, Info, X } from 'lucide-react';
import clsx from 'clsx';

export type ToastKind = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  message?: string;
}

interface ToastStore {
  toasts: Toast[];
  push: (t: Omit<Toast, 'id'>) => void;
  dismiss: (id: number) => void;
}

let seq = 1;

export const useToastStore = create<ToastStore>((set) => ({
  toasts: [],
  push: (t) => {
    const id = seq++;
    set((s) => ({ toasts: [...s.toasts.slice(-3), { ...t, id }] }));
    setTimeout(() => set((s) => ({ toasts: s.toasts.filter((x) => x.id !== id) })), t.kind === 'error' ? 6000 : 3500);
  },
  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((x) => x.id !== id) })),
}));

/** `const toast = useToast(); toast.success('Salvo')` */
export function useToast() {
  const push = useToastStore((s) => s.push);
  return {
    success: (title: string, message?: string) => push({ kind: 'success', title, message }),
    error: (title: string, message?: string) => push({ kind: 'error', title, message }),
    info: (title: string, message?: string) => push({ kind: 'info', title, message }),
  };
}

const ICONS = { success: CheckCircle2, error: CircleAlert, info: Info } as const;

export function Toaster() {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);
  return (
    <div className="pointer-events-none fixed right-4 bottom-4 z-[100] flex w-[340px] flex-col gap-2" aria-live="polite" role="status">
      <AnimatePresence>
        {toasts.map((t) => {
          const Icon = ICONS[t.kind];
          return (
            <motion.div
              key={t.id}
              initial={{ opacity: 0, y: 12, scale: 0.98 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 8, scale: 0.98 }}
              transition={{ duration: 0.18 }}
              className="card pointer-events-auto flex items-start gap-3 p-3 shadow-pop"
            >
              <Icon
                className={clsx('mt-0.5 size-4 shrink-0', {
                  'text-emerald-500': t.kind === 'success',
                  'text-red-500': t.kind === 'error',
                  'text-brand-500': t.kind === 'info',
                })}
              />
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium">{t.title}</p>
                {t.message && <p className="mt-0.5 text-xs text-ink-2">{t.message}</p>}
              </div>
              <button className="rounded-md p-1 text-ink-3 hover:bg-surface-2 hover:text-ink" onClick={() => dismiss(t.id)} aria-label="Fechar">
                <X className="size-3.5" />
              </button>
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}
