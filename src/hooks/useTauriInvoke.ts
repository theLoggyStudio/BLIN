import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";

interface InvokeState<T> {
  data: T | null;
  error: string | null;
  loading: boolean;
}

export function useTauriInvoke<T>() {
  const [state, setState] = useState<InvokeState<T>>({
    data: null,
    error: null,
    loading: false,
  });

  const execute = useCallback(
    async (command: string, args?: InvokeArgs): Promise<T> => {
    setState({ data: null, error: null, loading: true });
    try {
      const result = await invoke<T>(command, args);
      setState({ data: result, error: null, loading: false });
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setState({ data: null, error: message, loading: false });
      throw err;
    }
  },
    [],
  );

  return { ...state, execute };
}
