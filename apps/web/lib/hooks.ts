"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";

/** Fetches `path` now and every `everyMs` (0 = once). */
export function usePoll<T>(path: string, everyMs = 0) {
  const [data, setData] = useState<T>();
  const [error, setError] = useState<string>();
  const alive = useRef(true);
  const reload = useCallback(async () => {
    try {
      const d = await api<T>(path);
      if (alive.current) {
        setData(d);
        setError(undefined);
      }
    } catch (e) {
      if (alive.current) setError((e as Error).message);
    }
  }, [path]);
  useEffect(() => {
    alive.current = true;
    void reload();
    if (!everyMs) return () => void (alive.current = false);
    // Do not poll while the tab is hidden; refresh as soon as it is visible again.
    const t = setInterval(() => {
      if (document.visibilityState === "visible") void reload();
    }, everyMs);
    const onVisible = () => document.visibilityState === "visible" && void reload();
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      alive.current = false;
      clearInterval(t);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [reload, everyMs]);
  return { data, setData, error, reload };
}

/** Calls `fn` at most every `ms` while values keep coming (for sliders). */
export function useThrottle<A extends unknown[]>(fn: (...a: A) => void, ms = 120) {
  const last = useRef(0);
  const pending = useRef<ReturnType<typeof setTimeout>>(undefined);
  return useCallback(
    (...a: A) => {
      clearTimeout(pending.current);
      const wait = last.current + ms - Date.now();
      if (wait <= 0) {
        last.current = Date.now();
        fn(...a);
      } else {
        pending.current = setTimeout(() => {
          last.current = Date.now();
          fn(...a);
        }, wait);
      }
    },
    [fn, ms],
  );
}
