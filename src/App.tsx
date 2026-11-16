import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import {
  emptyExperience,
  projectLabel,
  type Cv,
  type Experience,
  type ExportAck,
  type Project,
  type ProjectSummary,
} from './cv';
import { sha256, showPdf } from './pdf';

type Preview = { revision: number; pages: number; sha256: string };
type RenderState = 'idle' | 'rendering' | 'ready' | 'failed';
type SaveState = 'idle' | 'waiting' | 'saving' | 'saved' | 'failed';

const SAVE_LABEL: Record<SaveState, string> = {
  idle: 'Not edited yet',
  waiting: 'Unsaved changes',
  saving: 'Saving…',
  saved: 'Saved',
  failed: 'Save failed',
};

export default function App() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [doc, setDoc] = useState<Project | null>(null);
  const [dirty, setDirty] = useState(false);
  const [saveState, setSaveState] = useState<SaveState>('idle');
  const [saveError, setSaveError] = useState('');
  const [preview, setPreview] = useState<Preview | null>(null);
  const [status, setStatus] = useState<RenderState>('idle');
  const [error, setError] = useState('');
  const [exported, setExported] = useState<(ExportAck & { matches: boolean }) | null>(null);

  const host = useRef<HTMLDivElement>(null);
  const revision = useRef(0);
  const latest = useRef(0);
  const edits = useRef(0);
  const saving = useRef(false);
  const savedOk = useRef(true);
  const docRef = useRef<Project | null>(null);
  docRef.current = doc;

  // --- library -------------------------------------------------------------

  const refreshLibrary = useCallback(async () => {
    try {
      setProjects(await invoke<ProjectSummary[]>('list_projects'));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refreshLibrary();
  }, [refreshLibrary]);

  const showProject = (project: Project) => {
    edits.current = 0;
    docRef.current = project;
    savedOk.current = true;
    setDoc(project);
    setDirty(false);
    setSaveState('idle');
    setSaveError('');
    setError('');
    setExported(null);
    setPreview(null);
    host.current?.replaceChildren();
  };

  const createProject = async () => {
    try {
      showProject(await invoke<Project>('create_project'));
      await refreshLibrary();
    } catch (e) {
      setError(String(e));
    }
  };

  const openProject = async (id: string) => {
    try {
      showProject(await invoke<Project>('open_project', { id }));
    } catch (e) {
      setError(String(e));
    }
  };

  const backToLibrary = async () => {
    if (dirty) await persist();
    if (!savedOk.current) {
      setError('Changes could not be saved. Retry the save before leaving this CV.');
      return;
    }
    setDoc(null);
    setDirty(false);
    setSaveState('idle');
    setPreview(null);
    setExported(null);
    host.current?.replaceChildren();
    await refreshLibrary();
  };

  // --- autosave: one second after the last edit -----------------------------

  const persist = useCallback(async () => {
    if (saving.current) return;
    const target = docRef.current;
    if (!target) return;
    const stamp = edits.current;
    saving.current = true;
    setSaveState('saving');
    setSaveError('');
    try {
      const stored = await invoke<Project>('save_project', { project: target });
      savedOk.current = true;
      setDoc((current) => (current && current.id === stored.id ? { ...current, updated_at: stored.updated_at } : current));
      if (edits.current === stamp) {
        setDirty(false);
        setSaveState('saved');
      }
    } catch (e) {
      savedOk.current = false;
      setSaveError(String(e));
      if (edits.current === stamp) setSaveState('failed');
    } finally {
      saving.current = false;
      // Edits that arrived during the write are still unsaved: never report them as saved.
      if (edits.current !== stamp) void persist();
    }
  }, []);

  useEffect(() => {
    if (!doc || !dirty) return;
    setSaveState((current) => (current === 'saving' ? current : 'waiting'));
    const timer = setTimeout(() => void persist(), 1000);
    return () => clearTimeout(timer);
  }, [doc, dirty, persist]);

  const edit = (change: (project: Project) => Project) => {
    setDoc((current) => (current ? change(current) : current));
    edits.current += 1;
    setDirty(true);
  };

  // --- preview -------------------------------------------------------------

  const render = useCallback(async (input: Cv) => {
    const rev = ++revision.current;
    latest.current = rev;
    setStatus('rendering');
    setError('');
    try {
      // Rust renders once and caches these exact bytes for export.
      const buffer = await invoke<ArrayBuffer>('render_pdf', { cv: input, revision: rev });
      if (latest.current !== rev) return; // late result from older content
      const bytes = new Uint8Array(buffer);
      const hash = await sha256(bytes);
      const pages = await showPdf(bytes, host.current!);
      if (latest.current !== rev) return;
      setPreview({ revision: rev, pages, sha256: hash });
      setStatus('ready');
    } catch (e) {
      if (latest.current !== rev) return;
      setError(String(e));
      setStatus('failed'); // older preview may stay visible, marked outdated
    }
  }, []);

  const cv = doc?.cv;
  useEffect(() => {
    if (!cv) {
      setStatus('idle');
      setPreview(null);
      return;
    }
    const timer = setTimeout(() => void render(cv), 400);
    return () => clearTimeout(timer);
  }, [cv, render]);

  const exportPdf = async () => {
    if (!preview || !doc) return;
    setExported(null);
    setError('');
    try {
      const path = await saveDialog({
        defaultPath: `${doc.cv.name.trim() || 'cv'}.pdf`,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
      if (!path) return;
      const ack = await invoke<ExportAck>('export_pdf', { revision: preview.revision, path, cv: doc.cv });
      setExported({ ...ack, matches: ack.sha256 === preview.sha256 });
    } catch (e) {
      setError(String(e));
    }
  };

  // --- views ---------------------------------------------------------------

  if (!doc) {
    return (
      <div className="app">
        <header className="bar">
          <div>
            <h1>CV Generator</h1>
            <p className="muted">Local library — projects stay on this device.</p>
          </div>
        </header>
        <main className="library">
          <div className="actions">
            <button type="button" onClick={() => void createProject()}>
              New CV
            </button>
            <button type="button" onClick={() => void refreshLibrary()}>
              Refresh
            </button>
          </div>

          {projects.length === 0 ? (
            <p className="muted">No CVs yet. Create one to start.</p>
          ) : (
            <ul className="list">
              {projects.map((project) => (
                <li key={project.id}>
                  <button type="button" className="row-button" onClick={() => void openProject(project.id)}>
                    <span>{projectLabel(project)}</span>
                    <span className="muted">{new Date(project.updated_at * 1000).toLocaleString()}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}

          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
        </main>
      </div>
    );
  }

  const setField = (field: 'name' | 'email' | 'phone' | 'location' | 'summary', value: string) =>
    edit((project) => ({ ...project, cv: { ...project.cv, [field]: value } }));

  const setExperience = (index: number, patch: Partial<Experience>) =>
    edit((project) => ({
      ...project,
      cv: {
        ...project.cv,
        experience: project.cv.experience.map((entry, i) => (i === index ? { ...entry, ...patch } : entry)),
      },
    }));

  const addExperience = () =>
    edit((project) => ({ ...project, cv: { ...project.cv, experience: [...project.cv.experience, emptyExperience()] } }));

  const removeExperience = (index: number) =>
    edit((project) => ({
      ...project,
      cv: { ...project.cv, experience: project.cv.experience.filter((_, i) => i !== index) },
    }));

  return (
    <div className="app">
      <header className="bar">
        <div>
          <h1>{projectLabel({ title: doc.title, name: doc.cv.name })}</h1>
          <p className="muted">Autosaves one second after you stop editing.</p>
        </div>
        <div className="status">
          <span className={`save ${saveState}`} role="status">
            {SAVE_LABEL[saveState]}
          </span>
          {saveState === 'failed' && (
            <button type="button" onClick={() => void persist()}>
              Retry save
            </button>
          )}
          <button type="button" onClick={() => void backToLibrary()}>
            Library
          </button>
        </div>
      </header>

      <main>
        <section className="form" aria-label="CV editor">
          <label>
            Project name (optional)
            <input
              value={doc.title}
              placeholder={doc.cv.name.trim() || 'Untitled CV'}
              onChange={(e) => edit((project) => ({ ...project, title: e.target.value }))}
            />
          </label>

          <h2 className="group">Contact details</h2>
          <label>
            Full name
            <input value={doc.cv.name} onChange={(e) => setField('name', e.target.value)} />
          </label>
          <label>
            Email
            <input value={doc.cv.email} onChange={(e) => setField('email', e.target.value)} />
          </label>
          <label>
            Phone (optional)
            <input value={doc.cv.phone} onChange={(e) => setField('phone', e.target.value)} />
          </label>
          <label>
            Location (optional)
            <input value={doc.cv.location} onChange={(e) => setField('location', e.target.value)} />
          </label>

          <h2 className="group">Professional summary</h2>
          <label>
            Summary
            <textarea rows={4} value={doc.cv.summary} onChange={(e) => setField('summary', e.target.value)} />
          </label>

          <h2 className="group">Work experience</h2>
          {doc.cv.experience.map((entry, index) => (
            <fieldset className="entry" key={index}>
              <legend>Role {index + 1}</legend>
              <label>
                Job title
                <input value={entry.title} onChange={(e) => setExperience(index, { title: e.target.value })} />
              </label>
              <label>
                Employer
                <input value={entry.company} onChange={(e) => setExperience(index, { company: e.target.value })} />
              </label>
              <label>
                Location
                <input value={entry.location} onChange={(e) => setExperience(index, { location: e.target.value })} />
              </label>
              <div className="row">
                <label>
                  Start (e.g. Jan 2020)
                  <input value={entry.start} onChange={(e) => setExperience(index, { start: e.target.value })} />
                </label>
                <label>
                  End
                  <input
                    value={entry.end ?? ''}
                    disabled={entry.end === null}
                    onChange={(e) => setExperience(index, { end: e.target.value })}
                  />
                </label>
              </div>
              <label className="check">
                <input
                  type="checkbox"
                  checked={entry.end === null}
                  onChange={(e) => setExperience(index, { end: e.target.checked ? null : '' })}
                />
                Ongoing (Present)
              </label>
              <label>
                Achievement bullets (one per line)
                <textarea
                  rows={3}
                  value={entry.bullets.join('\n')}
                  onChange={(e) => setExperience(index, { bullets: e.target.value.split('\n') })}
                />
              </label>
              <button type="button" onClick={() => removeExperience(index)}>
                Remove role
              </button>
            </fieldset>
          ))}
          <button type="button" className="add" onClick={addExperience}>
            Add work experience
          </button>

          <div className="actions">
            <button type="button" onClick={() => void render(doc.cv)} disabled={status === 'rendering'}>
              Render preview
            </button>
            <button type="button" onClick={() => void exportPdf()} disabled={!preview || status === 'failed'}>
              Export current PDF…
            </button>
          </div>

          <dl className="state">
            <dt>Save</dt>
            <dd>{SAVE_LABEL[saveState]}</dd>
            <dt>Render</dt>
            <dd>{status}</dd>
            <dt>Preview</dt>
            <dd>
              {preview
                ? `${preview.pages} page(s), revision ${preview.revision}${status === 'failed' ? ' (outdated)' : ''}`
                : 'none'}
            </dd>
            <dt>Preview SHA-256</dt>
            <dd className="hash">{preview?.sha256 ?? '—'}</dd>
            <dt>Export</dt>
            <dd>
              {exported
                ? `${exported.bytes} bytes → ${exported.path} — ${exported.matches ? 'identical bytes to preview' : 'DIFFERENT from preview'}`
                : 'not exported'}
            </dd>
            <dt>Export SHA-256</dt>
            <dd className="hash">{exported?.sha256 ?? '—'}</dd>
          </dl>

          {saveError && (
            <p className="error" role="alert">
              Save failed: {saveError}
            </p>
          )}

          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
        </section>

        <section className="preview" aria-label="PDF preview">
          {status === 'rendering' && <p className="muted">Rendering…</p>}
          <div ref={host} className="pages" />
        </section>
      </main>
    </div>
  );
}
