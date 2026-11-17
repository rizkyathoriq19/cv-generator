import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import {
  STRUCTURES,
  projectLabel,
  type Cv,
  type ExportAck,
  type Lang,
  type Opened,
  type Project,
  type ProjectSummary,
  type SaveResult,
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
  const [trash, setTrash] = useState<ProjectSummary[]>([]);
  const [showTrash, setShowTrash] = useState(false);
  // Permanent deletion waits for an explicit confirmation, never a row click.
  const [pendingDelete, setPendingDelete] = useState<ProjectSummary | null>(null);
  const [doc, setDoc] = useState<Project | null>(null);
  const [dirty, setDirty] = useState(false);
  const [saveState, setSaveState] = useState<SaveState>('idle');
  const [saveError, setSaveError] = useState('');
  const [preview, setPreview] = useState<Preview | null>(null);
  const [status, setStatus] = useState<RenderState>('idle');
  const [error, setError] = useState('');
  const [exported, setExported] = useState<(ExportAck & { matches: boolean }) | null>(null);
  const [backupStatus, setBackupStatus] = useState('');
  const [recovery, setRecovery] = useState('');
  // A close that could not save waits here until the user chooses how to go on.
  const [closePrompt, setClosePrompt] = useState<null | 'app' | 'library'>(null);
  // A permanent-deletion confirmation traps Tab between its two buttons.
  const cancelDelete = useRef<HTMLButtonElement>(null);
  const confirmDelete = useRef<HTMLButtonElement>(null);
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
  const saving = useRef<Promise<boolean> | null>(null);
  const docRef = useRef<Project | null>(null);
  docRef.current = doc;

  // --- library -------------------------------------------------------------

  const refreshLibrary = useCallback(async () => {
    try {
      const [active, trashed] = await Promise.all([
        invoke<ProjectSummary[]>('list_projects'),
        invoke<ProjectSummary[]>('list_trash'),
      ]);
      setProjects(active);
      setTrash(trashed);
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
    setDoc(project);
    setDirty(false);
    setSaveState('idle');
    setSaveError('');
    setError('');
    setRecovery('');
    setExported(null);
    setBackupStatus('');
    setPreview(null);
    setClosePrompt(null);
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
      const opened = await invoke<Opened>('open_project', { id, interfaceLanguage: lang });
      showProject(opened.project);
      // A damaged file is recovered from its previous revision with a notice;
      // without one, the open reports the damage instead of blanking the CV.
      setRecovery(opened.recovery?.notice ?? '');
    } catch (e) {
      setError(String(e));
    }
  };

  const leaveEditor = () => {
    // Clear the editor reference synchronously: a queued autosave must not write
    // after the user explicitly discarded the edits.
    docRef.current = null;
    setDoc(null);
    setDirty(false);
    setSaveState('idle');
    setSaveError('');
    setError('');
    setRecovery('');
    setPreview(null);
    setExported(null);
    setBackupStatus('');
    setClosePrompt(null);
    host.current?.replaceChildren();
    void refreshLibrary();
  };

  // --- autosave: one second after the last edit -----------------------------

  const persist = useCallback(async (): Promise<boolean> => {
    // Writes are serialized: a call made during a write waits for it, then writes
    // whatever is current, so edits made mid-write are never reported as saved.
    const run = (async () => {
      if (saving.current) await saving.current;
      const target = docRef.current;
      if (!target) return true;
      const stamp = edits.current;
      setSaveState('saving');
      setSaveError('');
      try {
        const result = await invoke<SaveResult>('save_project', {
          project: target,
          interfaceLanguage: langRef.current,
        });
        const stored = result.project;
        persisted.current = stamp;
        setDoc((current) => (current && current.id === stored.id ? { ...current, updated_at: stored.updated_at } : current));
        // A save that found a damaged file says so; it is never a silent replacement.
        if (result.notice) setRecovery(result.notice);
        if (edits.current === stamp) {
          setDirty(false);
          setSaveState('saved');
        }
        return true;
      } catch (e) {
        setSaveError(String(e));
        if (edits.current === stamp) setSaveState('failed');
        return false;
      }
    })();
    saving.current = run;
    try {
      return await run;
    } finally {
      if (saving.current === run) saving.current = null;
    }
  }, []);

  /**
   * Write until nothing is pending. A failed write stops the loop: only an
   * explicit discard may abandon edits that never reached the library.
   */
  const flush = useCallback(async (): Promise<boolean> => {
    while (docRef.current && edits.current > persisted.current) {
      if (!(await persist())) return false;
    }
    return true;
  }, [persist]);

  /** Normal close: save immediately, then finish the close the user asked for. */
  const finishClose = async (target: 'app' | 'library') => {
    setClosePrompt(null);
    if (target === 'app') {
      await getCurrentWindow().destroy();
      return;
    }
    leaveEditor();
  };

  const requestClose = async (target: 'app' | 'library') => {
    if (await flush()) {
      await finishClose(target);
      return;
    }
    setClosePrompt(target);
  };

  // --- Trash ----------------------------------------------------------------

  /** Delete resolves pending edits first; a failed flush keeps the CV in the library. */
  const moveToTrash = async (id: string) => {
    setError('');
    if (!(await flush())) return;
    try {
      await invoke('delete_project', { id, interfaceLanguage: lang });
      await refreshLibrary();
    } catch (e) {
      setError(String(e));
    }
  };

  const restoreProject = async (id: string) => {
    setError('');
    try {
      await invoke('restore_project', { id, interfaceLanguage: lang });
      await refreshLibrary();
    } catch (e) {
      setError(String(e));
    }
  };

  /** Only the confirmation dialog calls this: permanent deletion is never implicit. */
  const deletePermanently = async (id: string) => {
    setError('');
    try {
      await invoke('delete_project_permanently', { id, interfaceLanguage: lang });
      setPendingDelete(null);
      await refreshLibrary();
    } catch (e) {
      setError(String(e));
    }
  };

  // While the confirmation is open it owns the keyboard: Tab cycles its two
  // buttons and Escape cancels, even after a click moved focus to the dialog body.
  useEffect(() => {
    if (!pendingDelete) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setPendingDelete(null);
        return;
      }
      if (event.key !== 'Tab') return;
      event.preventDefault();
      const other =
        document.activeElement === cancelDelete.current ? confirmDelete.current : cancelDelete.current;
      other?.focus();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [pendingDelete]);

  // Closing the window is a normal close: pending edits are written first. If the
  // write fails, the window stays open and the close choices appear.
  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onCloseRequested(async (event) => {
        if (!docRef.current || edits.current <= persisted.current) return;
        event.preventDefault();
        if (await flush()) {
          await getCurrentWindow().destroy();
          return;
        }
        setClosePrompt('app');
      })
      .then((fn) => {
        if (alive) unlisten = fn;
        else fn();
      });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [flush]);

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
          <button
            type="button"
            aria-pressed={showTrash}
            onClick={() => {
              setError('');
              setShowTrash((current) => !current);
            }}
          >
            {t.trash} ({trash.length})
          </button>
        </div>
        <p className="muted note">{showTrash ? t.trashNote : t.backupNote}</p>

        {showTrash ? (
          trash.length === 0 ? (
            <p className="muted">{t.trashEmpty}</p>
          ) : (
            <ul className="list">
              {trash.map((project) => {
                const label = projectLabel(project, t.untitledCv);
                return (
                  <li key={project.id} className="project-row">
                    <span className="row-label">
                      <span>
                        {label}
                        {project.damaged && <span className="tag"> · {t.damaged}</span>}
                      </span>
                      {project.updated_at > 0 && (
                        <span className="muted">{new Date(project.updated_at * 1000).toLocaleString(dateLocale)}</span>
                      )}
                    </span>
                    <button
                      type="button"
                      aria-label={`${t.restore} ${label}`}
                      onClick={() => void restoreProject(project.id)}
                    >
                      {t.restore}
                    </button>
                    <button
                      type="button"
                      aria-label={`${t.deletePermanently} ${label}`}
                      onClick={() => {
                        setError('');
                        setPendingDelete(project);
                      }}
                    >
                      {t.deletePermanently}
                    </button>
                  </li>
                );
              })}
            </ul>
          )
        ) : projects.length === 0 ? (
          <p className="muted">{t.noCvs}</p>
        ) : (
          <ul className="list">
            {projects.map((project) => {
              const label = projectLabel(project, t.untitledCv);
              return (
                <li key={project.id} className="project-row">
                  <button
                    type="button"
                    className="row-button"
                    title={project.damaged ? t.damagedHint : undefined}
                    onClick={() => void openProject(project.id)}
                  >
                    <span>
                      {label}
                      {project.damaged && <span className="tag"> · {t.damaged}</span>}
                    </span>
                    {project.updated_at > 0 && (
                      <span className="muted">{new Date(project.updated_at * 1000).toLocaleString(dateLocale)}</span>
                    )}
                  </button>
                  <button
                    type="button"
                    aria-label={`${t.duplicate} ${label}`}
                    onClick={() => void duplicateProject(project.id)}
                  >
                    {t.duplicate}
                  </button>
                  <button
                    type="button"
                    aria-label={`${t.moveToTrash} ${label}`}
                    onClick={() => void moveToTrash(project.id)}
                  >
                    {t.moveToTrash}
                  </button>
                </li>
              );
            })}
          </ul>
        )}

        {!pendingDelete && error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
      </main>

      {pendingDelete && (
        <div
          className="overlay"
          role="dialog"
          aria-modal="true"
          aria-labelledby="delete-title"
        >
          <div className="dialog">
            <h2 id="delete-title">{t.deleteTitle}</h2>
            <p className="muted">{t.deleteBody}</p>
            <p>
              <strong>{projectLabel(pendingDelete, t.untitledCv)}</strong>
            </p>
            {error && (
              <p className="error" role="alert">
                {error}
              </p>
            )}
            <div className="actions">
              <button type="button" ref={cancelDelete} autoFocus onClick={() => setPendingDelete(null)}>
                {t.cancel}
              </button>
              <button type="button" ref={confirmDelete} onClick={() => void deletePermanently(pendingDelete.id)}>
                {t.deletePermanently}
              </button>
            </div>
          </div>
        </div>
      )}
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
          <button type="button" onClick={() => void requestClose('library')}>
            {t.library}
          </button>
        </div>
      </header>

      <main>
        <section className="form" aria-label={t.editorAria}>
          {recovery && (
            <p className="notice" role="status">
              {recovery}
            </p>
          )}

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

          {!closePrompt && saveError && (
            <p className="error" role="alert">
              {t.saveFailedPrefix} {saveError}
            </p>
          )}

          {!closePrompt && error && (
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

      {closePrompt && (
        <div
          className="overlay"
          role="dialog"
          aria-labelledby="close-title"
          onKeyDown={(event) => {
            if (event.key === 'Escape') setClosePrompt(null);
          }}
        >
          <div className="dialog">
            <h2 id="close-title">{t.closeTitle}</h2>
            <p className="muted">{t.closeBody}</p>
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
            {backupStatus && (
              <p className="muted" role="status">
                {t.backupExported}: {backupStatus}
              </p>
            )}
            <div className="actions">
              <button type="button" autoFocus onClick={() => void requestClose(closePrompt)}>
                {t.retrySave}
              </button>
              <button type="button" onClick={() => void exportBackup()}>
                {t.exportBackup}
              </button>
              <button type="button" onClick={() => void finishClose(closePrompt)}>
                {t.discardAndClose}
              </button>
              <button type="button" onClick={() => setClosePrompt(null)}>
                {t.cancel}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );

  return <LangContext.Provider value={{ lang, t }}>{view}</LangContext.Provider>;
}
