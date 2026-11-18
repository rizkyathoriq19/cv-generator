// MCP server over stdio: lets an agent render a CV. Newline-delimited JSON-RPC 2.0;
// stdout carries protocol messages only. Rendering is delegated to the desktop
// binary (`cv-generator --generate`) so validation and the Typst template stay
// identical to the app's PDF export. Run with `node mcp/server.ts` (Node >= 23.6).
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

const BIN =
  process.env.CV_GENERATOR_BIN ??
  fileURLToPath(new URL('../src-tauri/target/release/cv-generator', import.meta.url));

const s = { type: 'string' };
const strings = { type: 'array', items: s };
const date = {
  type: 'object',
  properties: {
    year: { type: ['integer', 'null'] },
    month: { type: ['integer', 'null'], minimum: 1, maximum: 12 },
  },
};
const list = (properties: object, description?: string) => ({
  type: 'array',
  ...(description && { description }),
  items: { type: 'object', properties },
});

const tool = {
  name: 'generate_cv',
  description:
    'Render an ATS-friendly CV to a text-based A4 PDF (and optionally an importable project backup). Bullets are user-authored achievement lines; dates use month/year, year only, or empty.',
  inputSchema: {
    type: 'object',
    required: ['cv', 'output_path'],
    properties: {
      output_path: { type: 'string', description: 'Absolute path of the .pdf to write (replaced atomically).' },
      backup_path: {
        type: 'string',
        description: 'Optional absolute .json path: a project backup the desktop app can import for later editing.',
      },
      title: { type: 'string', description: 'Project title stored in the backup.' },
      structure: {
        type: 'string',
        enum: ['reverse-chronological', 'functional', 'combination'],
        default: 'reverse-chronological',
      },
      content_language: {
        type: 'string',
        enum: ['english', 'indonesian'],
        default: 'english',
        description: "Decides standard headings and 'Present' wording; content is not translated.",
      },
      cv: {
        type: 'object',
        required: ['name'],
        properties: {
          name: s, headline: s, email: s, phone: s, location: s, summary: s,
          links: list({ label: s, url: s }),
          experience: list({
            title: s, company: s, location: s, start: date,
            end: { description: 'null = Present (ongoing role)', anyOf: [date, { type: 'null' }] },
            bullets: strings,
          }),
          education: list({
            school: s, degree: s, location: s, start: date,
            end: { anyOf: [date, { type: 'null' }] }, bullets: strings,
          }),
          skills: list({ name: s, bullets: strings }, 'Skill groups; lead functional/combination CVs.'),
          projects: list({ name: s, detail: s, bullets: strings }),
          certifications: list({ name: s, issuer: s, date }),
          languages: list({ name: s, level: s }),
        },
      },
    },
  },
};

/** Runs the binary once; resolves its stdout report, rejects with its stderr. */
function generate(args: unknown): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn(BIN, ['--generate'], { stdio: ['pipe', 'pipe', 'pipe'] });
    let out = '';
    let err = '';
    child.stdout.on('data', (d) => (out += d));
    child.stderr.on('data', (d) => (err += d));
    child.on('error', (e) => reject(new Error(`cannot run ${BIN}: ${e.message}`)));
    child.on('close', (code) => (code === 0 ? resolve(out.trim()) : reject(new Error(err.trim() || `exit ${code}`))));
    child.stdin.end(JSON.stringify(args ?? {}));
  });
}

type Message = { id?: unknown; method?: string; params?: any };

const error = (id: unknown, code: number, message: string) => ({ jsonrpc: '2.0', id, error: { code, message } });

/** One JSON-RPC message in, at most one response out (notifications get none). */
export async function handle(msg: Message): Promise<object | undefined> {
  if (msg.id === undefined) return undefined;
  const { id, params = {} } = msg;
  let result: object;
  switch (msg.method) {
    case 'initialize':
      result = {
        protocolVersion: params.protocolVersion ?? '2025-06-18',
        capabilities: { tools: {} },
        serverInfo: { name: 'cv-generator', version: '0.1.0' },
      };
      break;
    case 'ping':
      result = {};
      break;
    case 'tools/list':
      result = { tools: [tool] };
      break;
    case 'tools/call':
      if (params.name !== tool.name) return error(id, -32602, 'unknown tool');
      try {
        result = { content: [{ type: 'text', text: await generate(params.arguments) }] };
      } catch (e) {
        result = { content: [{ type: 'text', text: (e as Error).message }], isError: true };
      }
      break;
    default:
      return error(id, -32601, 'method not found');
  }
  return { jsonrpc: '2.0', id, result };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  for await (const line of createInterface({ input: process.stdin })) {
    if (!line.trim()) continue;
    let reply;
    try {
      reply = await handle(JSON.parse(line));
    } catch {
      reply = error(null, -32700, 'parse error');
    }
    if (reply) process.stdout.write(JSON.stringify(reply) + '\n');
  }
}
