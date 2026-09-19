import { useCallback, useEffect, useRef, useState } from 'react';

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

/** Small data-fetching hook: reruns `fn` whenever `deps` change; exposes `reload`. */
export function useAsync<T>(fn: () => Promise<T>, deps: unknown[]): AsyncState<T> & { reload: () => Promise<void>; setData: (d: T) => void } {
  const [state, setState] = useState<AsyncState<T>>({ data: null, loading: true, error: null });
  const seq = useRef(0);
  const fnRef = useRef(fn);
  fnRef.current = fn;

  const reload = useCallback(async () => {
    const id = ++seq.current;
    setState((s) => ({ ...s, loading: s.data === null, error: null }));
    try {
      const data = await fnRef.current();
      if (id === seq.current) setState({ data, loading: false, error: null });
    } catch (e) {
      if (id === seq.current) setState((s) => ({ ...s, loading: false, error: e instanceof Error ? e.message : String(e) }));
    }
  }, []);

  useEffect(() => {
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  const setData = useCallback((d: T) => setState({ data: d, loading: false, error: null }), []);

  return { ...state, reload, setData };
}
