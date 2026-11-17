import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import {
  STRUCTURES,
  projectLabel,
  type Cv,
  type ExportAck,
  type Lang,
  type Project,
  type ProjectSummary,
  type Structure,
} from './cv';
import {
  CertificationsSection,
  EducationSection,
  ExperienceSection,
  LanguagesSection,
  LinksSection,
  ProjectsSection,
  SkillsSection,
} from './Sections';
import { sha256, showPdf } from './pdf';
import { LANGS, LangContext, STRINGS, saveLang, storedLang, type Strings } from './i18n';

type Preview = { revision: number; pages: number; sha256: string };
type RenderState = 'idle' | 'rendering' | 'ready' | 'failed';
type SaveState = 'idle' | 'waiting' | 'saving' | 'saved' | 'failed';

const SAVE_KEY: Record<SaveState, keyof Strings> = {
  idle: 'saveIdle',
  waiting: 'saveWaiting',
  saving: 'saveSaving',
  saved: 'saveSaved',
  failed: 'saveFailed',
};

const STATUS_KEY: Record<RenderState, keyof Strings> = {
  idle: 'statusIdle',
  rendering: 'statusRendering',
  ready: 'statusReady',
  failed: 'statusFailed',
};

const STRUCTURE_KEY: Record<Structure, keyof Strings> = {
  'reverse-chronological': 'structureReverse',
  functional: 'structureFunctional',
  combination: 'structureCombination',
};

/** Language names stay native, so the choice is readable in either interface. */
function LanguageChoice({
  label,
  value,
  onChange,
}: {
  label: string;
  value: Lang;
  onChange: (next: Lang) => void;
}) {
  return (
    <label className="lang">
      {label}
      <select value={value} onChange={(e) => onChange(e.target.value as Lang)}>
        {LANGS.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

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
  const [backupStatus, setBackupStatus] = useState('');
  const [lang, setLang] = useState<Lang>(storedLang);

  const t = STRINGS[lang];
  // The reader's language, readable from callbacks that must not re-run when it
  // changes: an interface switch never re-renders or edits the CV.
  const langRef = useRef(lang);
  langRef.current = lang;
  const dateLocale = lang === 'indonesian' ? 'id-ID' : 'en-US';

  // Interface language is an application preference: it never touches CV data.
  useEffect(() => {
    saveLang(lang);
    document.documentElement.lang = lang === 'indonesian' ? 'id' : 'en';
  }, [lang]);

  const host = useRef<HTMLDivElement>(null);
  const revision = useRef(0);
  const latest = useRef(0);
  const edits = useRef(0);
  const persisted = useRef(0); // edit count at the last successful write
  const saving = useRef<Promise<void> | null>(null);
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
    persisted.current = 0;
    docRef.current = project;
    savedOk.current = true;
    setDoc(project);
    setDirty(false);
    setSaveState('idle');
    setSaveError('');
    setError('');
    setExported(null);
    setBackupStatus('');
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

  /** Duplication copies content, structure, and content language into a new CV. */
  const duplicateProject = async (id: string) => {
    try {
      showProject(await invoke<Project>('duplicate_project', { id }));
      await refreshLibrary();
    } catch (e) {
      setError(String(e));
    }
  };

  /** Import always creates a new CV; the backup file and existing CVs are never modified. */
  const importBackup = async () => {
    setError('');
    try {
      const path = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: t.backupFile, extensions: ['json'] }],
      });
      if (!path) return;
      showProject(await invoke<Project>('import_backup', { path, interfaceLanguage: lang }));
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
    // Wait out any in-flight write, then keep writing until no edit is unsaved:
    // a save started before the newest keystroke must not end the session silently.
    while (docRef.current && savedOk.current && edits.current > persisted.current) await persist();
    if (edits.current > persisted.current) {
      setError(t.cannotSave);
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
    // Writes are serialized: a call made during a write waits for it, then writes
    // whatever is current, so edits made mid-write are never reported as saved.
    const run = (async () => {
      if (saving.current) await saving.current;
      const target = docRef.current;
      if (!target) return;
      const stamp = edits.current;
      setSaveState('saving');
      setSaveError('');
      try {
        const stored = await invoke<Project>('save_project', { project: target });
        savedOk.current = true;
        persisted.current = stamp;
        setDoc((current) => (current && current.id === stored.id ? { ...current, updated_at: stored.updated_at } : current));
        if (edits.current === stamp) {
          setDirty(false);
          setSaveState('saved');
        }
      } catch (e) {
        savedOk.current = false;
        setSaveError(String(e));
        if (edits.current === stamp) setSaveState('failed');
      }
    })();
    saving.current = run;
    try {
      await run;
    } finally {
      if (saving.current === run) saving.current = null;
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

  const editCv = (change: (cv: Cv) => Cv) => edit((project) => ({ ...project, cv: change(project.cv) }));

  // --- preview -------------------------------------------------------------

  const render = useCallback(async (input: Cv, structure: Structure, contentLanguage: Lang) => {
    const rev = ++revision.current;
    latest.current = rev;
    setStatus('rendering');
    setError('');
    try {
      // Rust renders once and caches these exact bytes for export.
      const buffer = await invoke<ArrayBuffer>('render_pdf', {
        cv: input,
        structure,
        contentLanguage,
        interfaceLanguage: langRef.current,
        revision: rev,
      });
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
  const structure = doc?.structure;
  const contentLanguage = doc?.content_language;
  useEffect(() => {
    if (!cv || !structure || !contentLanguage) {
      setStatus('idle');
      setPreview(null);
      return;
    }
    const timer = setTimeout(() => void render(cv, structure, contentLanguage), 400);
    return () => clearTimeout(timer);
  }, [cv, structure, contentLanguage, render]);

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
      const ack = await invoke<ExportAck>('export_pdf', {
        revision: preview.revision,
        path,
        cv: doc.cv,
        structure: doc.structure,
        contentLanguage: doc.content_language,
        interfaceLanguage: lang,
      });
      setExported({ ...ack, matches: ack.sha256 === preview.sha256 });
    } catch (e) {
      setError(String(e));
    }
  };

  const exportBackup = async () => {
    if (!doc) return;
    setBackupStatus('');
    setError('');
    try {
      const path = await saveDialog({
        defaultPath: `${doc.cv.name.trim() || 'cv'}-backup.json`,
        filters: [{ name: t.backupFile, extensions: ['json'] }],
      });
      if (!path) return;
      const ack = await invoke<ExportAck>('export_backup', {
        path,
        title: doc.title,
        structure: doc.structure,
        contentLanguage: doc.content_language,
        cv: doc.cv,
        interfaceLanguage: lang,
      });
      setBackupStatus(`${ack.bytes} bytes → ${ack.path}`);
    } catch (e) {
      setError(String(e));
    }
  };

  // --- views ---------------------------------------------------------------

  const setField = (field: 'name' | 'email' | 'phone' | 'location' | 'summary', value: string) =>
    editCv((cv) => ({ ...cv, [field]: value }));

  const view = !doc ? (
    <div className="app">
      <header className="bar">
        <div>
          <h1>CV Generator</h1>
          <p className="muted">{t.libraryTagline}</p>
        </div>
        <LanguageChoice label={t.interfaceShort} value={lang} onChange={setLang} />
      </header>
      <main className="library">
        <div className="actions">
          <button type="button" onClick={() => void createProject()}>
            {t.newCv}
          </button>
          <button type="button" onClick={() => void refreshLibrary()}>
            {t.refresh}
          </button>
          <button type="button" onClick={() => void importBackup()}>
            {t.importBackup}
          </button>
        </div>
        <p className="muted note">{t.backupNote}</p>

        {projects.length === 0 ? (
          <p className="muted">{t.noCvs}</p>
        ) : (
          <ul className="list">
            {projects.map((project) => {
              const label = projectLabel(project, t.untitledCv);
              return (
                <li key={project.id} className="project-row">
                  <button type="button" className="row-button" onClick={() => void openProject(project.id)}>
                    <span>{label}</span>
                    <span className="muted">{new Date(project.updated_at * 1000).toLocaleString(dateLocale)}</span>
                  </button>
                  <button
                    type="button"
                    aria-label={`${t.duplicate} ${label}`}
                    onClick={() => void duplicateProject(project.id)}
                  >
                    {t.duplicate}
                  </button>
                </li>
              );
            })}
          </ul>
        )}

        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
      </main>
    </div>
  ) : (
    <div className="app">
      <header className="bar">
        <div>
          <h1>{projectLabel({ title: doc.title, name: doc.cv.name }, t.untitledCv)}</h1>
          <p className="muted">{t.editorTagline}</p>
        </div>
        <div className="status">
          <span className={`save ${saveState}`} role="status">
            {t[SAVE_KEY[saveState]]}
          </span>
          {saveState === 'failed' && (
            <button type="button" onClick={() => void persist()}>
              {t.retrySave}
            </button>
          )}
          <LanguageChoice label={t.interfaceShort} value={lang} onChange={setLang} />
          <button type="button" onClick={() => void backToLibrary()}>
            {t.library}
          </button>
        </div>
      </header>

      <main>
        <section className="form" aria-label={t.editorAria}>
          <label>
            {t.projectNameOptional}
            <input
              value={doc.title}
              placeholder={doc.cv.name.trim() || t.untitledCv}
              onChange={(e) => edit((project) => ({ ...project, title: e.target.value }))}
            />
          </label>

          <label>
            {t.contentStructure}
            <select
              value={doc.structure}
              onChange={(e) => edit((project) => ({ ...project, structure: e.target.value as Structure }))}
            >
              {STRUCTURES.map((value) => (
                <option key={value} value={value}>
                  {t[STRUCTURE_KEY[value]]}
                </option>
              ))}
            </select>
          </label>
          <p className="muted">{t.structureHint}</p>

          <label>
            {t.contentLanguage}
            <select
              value={doc.content_language}
              onChange={(e) => edit((project) => ({ ...project, content_language: e.target.value as Lang }))}
            >
              {LANGS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <p className="muted">{t.contentLanguageHint}</p>

          <h2 className="group">{t.contactDetails}</h2>
          <label>
            {t.fullName}
            <input value={doc.cv.name} onChange={(e) => setField('name', e.target.value)} />
          </label>
          <label>
            {t.email}
            <input value={doc.cv.email} onChange={(e) => setField('email', e.target.value)} />
          </label>
          <label>
            {t.phoneOptional}
            <input value={doc.cv.phone} onChange={(e) => setField('phone', e.target.value)} />
          </label>
          <label>
            {t.locationOptional}
            <input value={doc.cv.location} onChange={(e) => setField('location', e.target.value)} />
          </label>

          <h2 className="group">{t.linksOptional}</h2>
          <LinksSection items={doc.cv.links} onChange={(links) => editCv((cv) => ({ ...cv, links }))} />

          <h2 className="group">{t.professionalSummary}</h2>
          <label>
            {t.summary}
            <textarea rows={4} value={doc.cv.summary} onChange={(e) => setField('summary', e.target.value)} />
          </label>

          <h2 className="group">{t.workExperience}</h2>
          <ExperienceSection
            items={doc.cv.experience}
            onChange={(experience) => editCv((cv) => ({ ...cv, experience }))}
          />

          <h2 className="group">{t.education}</h2>
          <EducationSection
            items={doc.cv.education}
            onChange={(education) => editCv((cv) => ({ ...cv, education }))}
          />

          <h2 className="group">{t.skills}</h2>
          <SkillsSection items={doc.cv.skills} onChange={(skills) => editCv((cv) => ({ ...cv, skills }))} />

          <h2 className="group">{t.projects}</h2>
          <ProjectsSection items={doc.cv.projects} onChange={(projects) => editCv((cv) => ({ ...cv, projects }))} />

          <h2 className="group">{t.certifications}</h2>
          <CertificationsSection
            items={doc.cv.certifications}
            onChange={(certifications) => editCv((cv) => ({ ...cv, certifications }))}
          />

          <h2 className="group">{t.languages}</h2>
          <LanguagesSection items={doc.cv.languages} onChange={(languages) => editCv((cv) => ({ ...cv, languages }))} />

          <div className="actions">
            <button
              type="button"
              onClick={() => void render(doc.cv, doc.structure, doc.content_language)}
              disabled={status === 'rendering'}
            >
              {t.renderPreview}
            </button>
            <button type="button" onClick={() => void exportPdf()} disabled={!preview || status === 'failed'}>
              {t.exportPdf}
            </button>
            <button type="button" onClick={() => void exportBackup()}>
              {t.exportBackup}
            </button>
          </div>
          <p className="muted note">{t.backupNote}</p>
          {backupStatus && (
            <p className="muted" role="status">
              {t.backupExported}: {backupStatus}
            </p>
          )}

          <dl className="state">
            <dt>{t.save}</dt>
            <dd>{t[SAVE_KEY[saveState]]}</dd>
            <dt>{t.render}</dt>
            <dd>{t[STATUS_KEY[status]]}</dd>
            <dt>{t.preview}</dt>
            <dd>
              {preview
                ? `${preview.pages} ${t.pageCount}, ${t.revision} ${preview.revision}${
                    status === 'failed' ? ` ${t.outdated}` : ''
                  }`
                : t.none}
            </dd>
            <dt>{t.previewHash}</dt>
            <dd className="hash">{preview?.sha256 ?? '—'}</dd>
            <dt>{t.export}</dt>
            <dd>
              {exported
                ? `${exported.bytes} bytes → ${exported.path} — ${
                    exported.matches ? t.identicalBytes : t.differentBytes
                  }`
                : t.notExported}
            </dd>
            <dt>{t.exportHash}</dt>
            <dd className="hash">{exported?.sha256 ?? '—'}</dd>
          </dl>

          {exported && exported.warnings.length > 0 && (
            <ul className="warning" role="status">
              {exported.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          )}

          {saveError && (
            <p className="error" role="alert">
              {t.saveFailedPrefix} {saveError}
            </p>
          )}

          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
        </section>

        <section className="preview" aria-label={t.previewAria}>
          {status === 'rendering' && <p className="muted">{t.rendering}</p>}
          <div ref={host} className="pages" />
        </section>
      </main>
    </div>
  );

  return <LangContext.Provider value={{ lang, t }}>{view}</LangContext.Provider>;
}
