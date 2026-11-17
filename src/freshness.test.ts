import test from 'node:test';
import assert from 'node:assert/strict';
import { emptyCv } from './cv.ts';
import { canReusePreview, contentKey, previewState, type RenderInput } from './freshness.ts';

const input = (patch: Partial<RenderInput> = {}): RenderInput => ({
  cv: { ...emptyCv(), name: 'Ada Lovelace', summary: 'Analytical engine programmer.' },
  structure: 'reverse-chronological',
  content_language: 'english',
  ...patch,
});

test('an edit changes the key, so the old preview is outdated', () => {
  const rendered = contentKey(input());
  const edited = contentKey({ ...input(), cv: { ...input().cv, summary: 'Edited after the preview.' } });
  assert.notEqual(edited, rendered);
  assert.equal(previewState({ key: rendered }, rendered), 'current');
  assert.equal(previewState({ key: rendered }, edited), 'outdated');
  assert.equal(previewState(null, edited), 'none');
});

test('structure and content language are part of the preview identity', () => {
  const rendered = contentKey(input());
  assert.notEqual(rendered, contentKey({ ...input(), structure: 'functional' }));
  assert.notEqual(rendered, contentKey({ ...input(), content_language: 'indonesian' }));
});

test('export reuses preview bytes only while they are current and not mid-render', () => {
  const rendered = contentKey(input());
  const preview = { key: rendered };
  assert.equal(canReusePreview(preview, rendered, false), true, 'current bytes are reused');
  assert.equal(canReusePreview(preview, rendered, true), false, 'an in-flight render is not reused');
  const edited = contentKey({ ...input(), structure: 'combination' });
  assert.equal(canReusePreview(preview, edited, false), false, 'a stale preview must be rendered again');
  assert.equal(canReusePreview(null, edited, false), false, 'nothing rendered yet');
});
