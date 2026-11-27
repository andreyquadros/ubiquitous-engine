import { useCallback, useEffect, useState } from 'react';
import { hostIpc } from './hostIpc';

/** "Abrir junto com o computador", read from the system's login item; `undefined` until it is known. */
export function useAutostart(): [boolean | undefined, (on: boolean) => Promise<void>] {
  const [value, setValue] = useState<boolean | undefined>(undefined);
  useEffect(() => {
    let live = true;
    void hostIpc
      .autostart()
      .then((on) => live && setValue(on))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);
  const change = useCallback(async (on: boolean) => {
    await hostIpc.setAutostart(on);
    setValue(on);
  }, []);
  return [value, change];
}
