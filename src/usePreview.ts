import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Lang } from './cv';
import { contentKey, previewState, type RenderInput } from './freshness';
import { pdfPages, sha256 } from './pdf';

export type Preview = { revision: number; pages: number; sha256: string; key: string };
export type RenderState = 'idle' | 'rendering' | 'ready' | 'failed';

/** Pause after the last edit before the PDF regenerates. */
const DEBOUNCE_MS = 400;

/**
 * The generated PDF of the CV being edited. Renders after a short pause, lets
 * only the newest render reach the screen, and reports older pages as outdated
 * while they stay visible. `render` is also the export path's regeneration.
 */
export function usePreview(doc: RenderInput | null, interfaceLanguage: RefObject<Lang>) {
  const host = useRef<HTMLDivElement>(null);
  const latest = useRef(0);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [status, setStatus] = useState<RenderState>('idle');
  const [error, setError] = useState('');

  const render = useCallback(
    async ({ cv, structure, content_language }: RenderInput): Promise<Preview | null> => {
      const rev = ++latest.current;
      // The content the bytes belong to: any later edit, structure, or content
      // language change makes this key differ and marks the preview outdated.
      const key = contentKey({ cv, structure, content_language });
      setStatus('rendering');
      setError('');
      try {
        // Rust renders once and caches these exact bytes for export.
        const buffer = await invoke<ArrayBuffer>('render_pdf', {
          cv,
          structure,
          contentLanguage: content_language,
          interfaceLanguage: interfaceLanguage.current,
          revision: rev,
        });
        if (latest.current !== rev) return null; // late result from older content
        const bytes = new Uint8Array(buffer);
        const hash = await sha256(bytes);
        const { pages, canvases } = await pdfPages(bytes);
        // Only the newest render may replace what is on screen, pages included.
        if (latest.current !== rev) return null;
        host.current?.replaceChildren(...canvases);
        const done = { revision: rev, pages, sha256: hash, key };
        setPreview(done);
        setStatus('ready');
        return done;
      } catch (e) {
        if (latest.current !== rev) return null;
        setError(String(e));
        setStatus('failed'); // older preview may stay visible, marked outdated
        return null;
      }
    },
    [interfaceLanguage],
  );

  const cv = doc?.cv;
  const structure = doc?.structure;
  const contentLanguage = doc?.content_language;
  useEffect(() => {
    if (!cv || !structure || !contentLanguage) {
      setStatus('idle');
      setPreview(null);
      return;
    }
    const timer = setTimeout(() => void render({ cv, structure, content_language: contentLanguage }), DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [cv, structure, contentLanguage, render]);

  // A doc edit before the debounce fires already makes the visible bytes stale.
  const stale = previewState(preview, doc ? contentKey(doc) : '') === 'outdated';

  return { host, preview, status, error, stale, render };
}
