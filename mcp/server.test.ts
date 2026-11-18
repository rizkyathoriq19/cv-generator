import test from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// Stand-in for `cv-generator --generate`: succeeds unless the input mentions "bad".
const dir = mkdtempSync(join(tmpdir(), 'cvgen-mcp-'));
const bin = join(dir, 'fake-cv-generator');
writeFileSync(bin, '#!/bin/sh\nin=$(cat)\ncase "$in" in *bad*) echo "name is required" >&2; exit 1;; esac\necho \'{"path":"/x.pdf"}\'\n');
chmodSync(bin, 0o755);
process.env.CV_GENERATOR_BIN = bin;
const { handle } = await import('./server.ts');

const call = (args: object) =>
  handle({ id: 1, method: 'tools/call', params: { name: 'generate_cv', arguments: args } }) as Promise<any>;

test('tools/list exposes generate_cv', async () => {
  const reply = (await handle({ id: 1, method: 'tools/list' })) as any;
  assert.equal(reply.result.tools[0].name, 'generate_cv');
});

test('a successful render returns the binary report', async () => {
  const reply = await call({ cv: { name: 'Ada' } });
  assert.equal(reply.result.isError, undefined);
  assert.equal(reply.result.content[0].text, '{"path":"/x.pdf"}');
});

test('a failed render is a tool error carrying stderr', async () => {
  const reply = await call({ cv: { name: 'bad' } });
  assert.equal(reply.result.isError, true);
  assert.equal(reply.result.content[0].text, 'name is required');
});

test('notifications get no reply; unknown methods and tools are protocol errors', async () => {
  assert.equal(await handle({ method: 'notifications/initialized' }), undefined);
  assert.equal(((await handle({ id: 2, method: 'nope' })) as any).error.code, -32601);
  assert.equal(((await handle({ id: 3, method: 'tools/call', params: { name: 'x' } })) as any).error.code, -32602);
});
