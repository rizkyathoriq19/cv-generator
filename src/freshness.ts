import type { Cv, Lang, Structure } from './cv.ts';

/** The content a preview's bytes were rendered from. */
export type RenderInput = { cv: Cv; structure: Structure; content_language: Lang };

/**
 * Identity of rendered content: an edit, a structure switch, or a content
 * language change produces a new key, so an older artifact cannot pass as
 * current and export can tell whether it may reuse the previewed bytes.
 */
export const contentKey = ({ cv, structure, content_language }: RenderInput): string =>
  JSON.stringify([structure, content_language, cv]);

/**
 * `current` — the visible bytes were rendered from exactly this content.
 * `outdated` — an older preview is still on screen and must be labelled so.
 * `none` — nothing has been rendered for this editor yet.
 */
export type PreviewState = 'none' | 'current' | 'outdated';

export const previewState = (preview: { key: string } | null, key: string): PreviewState =>
  preview === null ? 'none' : preview.key === key ? 'current' : 'outdated';

/**
 * Export reuses the previewed artifact only while it was rendered from the
 * current content and no render is in flight: stale bytes are regenerated first.
 */
export const canReusePreview = (
  preview: { key: string } | null,
  key: string,
  rendering: boolean,
): preview is { key: string } => preview !== null && preview.key === key && !rendering;
