import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { STRUCTURES, projectLabel, type Cv, type ExportAck, type Lang, type Project, type SaveResult } from './cv';
import {
  CertificationsSection,
  EducationSection,
  ExperienceSection,
  LanguagesSection,
  LinksSection,
  ProjectsSection,
  SkillsSection,
} from './Sections';
import { canReusePreview, contentKey } from './freshness';
import { LANGS, STRUCTURE_KEY, useT, type Strings } from './i18n';
import { usePreview, type RenderState } from './usePreview';

type SaveState = 'idle' | 'waiting' | 'saving' | 'saved' | 'failed';
type Pane = 'form' | 'preview';
type Exported = ExportAck & { matches: boolean; changed: boolean };

/** Below this width the editor and the preview are tabs, not columns. Keep in sync with styles.css. */
const NARROW = '(max-width: 900px)';
/** Autosave one second after the last edit. */
const AUTOSAVE_MS = 1000;

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

/** True while the window is too narrow for a side-by-side editor and preview. */
function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => window.matchMedia(NARROW).matches);
  useEffect(() => {
    const media = window.matchMedia(NARROW);
    const update = () => setNarrow(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);
  return narrow;
}

/** One titled block of the form; the id is the jump-navigation target. */
function Card({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <section className="card" id={id} aria-labelledby={`${id}-title`}>
      <h2 id={`${id}-title`}>{title}</h2>
      {children}
    </section>
  );
}

/**
 * The editor for one open CV. It is mounted per project, so opening another CV
 * starts from clean state; leaving goes through `requestClose`, which saves first.
 */
export default function Editor({
  initial,
  recoveryNotice = '',
  languageChoice,
  onLeave,
}: {
  initial: Project;
  recoveryNotice?: string;
  languageChoice: ReactNode;
  onLeave: () => void;
}) {
  const { lang, t } = useT();
  const [doc, setDoc] = useState<Project>(initial);
  const [dirty, setDirty] = useState(false);
  const [saveState, setSaveState] = useState<SaveState>('idle');
  const [saveError, setSaveError] = useState('');
  const [error, setError] = useState('');
  const [recovery, setRecovery] = useState(recoveryNotice);
  const [exported, setExported] = useState<Exported | null>(null);
  const [backupStatus, setBackupStatus] = useState('');
  // A close that could not save waits here until the user chooses how to go on.
  const [closePrompt, setClosePrompt] = useState<null | 'app' | 'library'>(null);
  const narrow = useNarrow();
  // Narrow windows show one pane at a time; wide ones show both together.
  const [pane, setPane] = useState<Pane>('form');

  // The reader's language, readable from callbacks that must not re-run when it
  // changes: an interface switch never re-renders or edits the CV.
  const langRef = useRef(lang);
  langRef.current = lang;

  // `null` once the editor closed: a queued autosave must not write after a discard.
  const closed = useRef(false);
  const docRef = useRef<Project | null>(doc);
  docRef.current = closed.current ? null : doc;
  const edits = useRef(0);
  const persisted = useRef(0); // edit count at the last successful write
  const saving = useRef<Promise<boolean> | null>(null);

  const { host, preview, status, error: renderError, stale, render } = usePreview(doc, langRef);

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
        persisted.current = stamp;
        setDoc((current) => ({ ...current, updated_at: result.project.updated_at }));
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
    // Mark the editor closed synchronously so no queued autosave writes after a discard.
    closed.current = true;
    docRef.current = null;
    if (target === 'app') {
      await getCurrentWindow().destroy();
      return;
    }
    onLeave();
  };

  const requestClose = async (target: 'app' | 'library') => {
    if (await flush()) {
      await finishClose(target);
      return;
    }
    setClosePrompt(target);
  };

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
    if (!dirty) return;
    setSaveState((current) => (current === 'saving' ? current : 'waiting'));
    const timer = setTimeout(() => void persist(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  }, [doc, dirty, persist]);

  const edit = (change: (project: Project) => Project) => {
    setDoc(change);
    edits.current += 1;
    setDirty(true);
  };

  const editCv = (change: (cv: Cv) => Cv) => edit((project) => ({ ...project, cv: change(project.cv) }));

  // --- export ----------------------------------------------------------------

  /**
   * Export ships the previewed artifact only while it is the current content;
   * otherwise it generates and shows the current revision first. A failed render
   * leaves the older preview visibly outdated and writes nothing.
   */
  const exportPdf = async () => {
    setExported(null);
    setError('');
    const target = contentKey(doc);
    const fresh = canReusePreview(preview, target, status === 'rendering') ? preview : await render(doc);
    if (!fresh) return;
    try {
      const path = await saveDialog({
        defaultPath: `${doc.cv.name.trim() || 'cv'}.pdf`,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
      if (!path) return;
      const ack = await invoke<ExportAck>('export_pdf', {
        path,
        cv: doc.cv,
        structure: doc.structure,
        contentLanguage: doc.content_language,
        interfaceLanguage: lang,
      });
      // An edit after this export started means the saved file is not, and never
      // was, the current CV: say so instead of presenting it as current.
      const changed = !docRef.current || contentKey(docRef.current) !== target;
      setExported({ ...ack, matches: ack.sha256 === fresh.sha256, changed });
    } catch (e) {
      setError(String(e));
    }
  };

  const exportBackup = async () => {
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

  // --- view ------------------------------------------------------------------

  const setField = (field: 'name' | 'headline' | 'email' | 'phone' | 'location' | 'summary', value: string) =>
    editCv((cv) => ({ ...cv, [field]: value }));

  const sections: { id: string; title: string }[] = [
    { id: 'sec-contact', title: t.contactDetails },
    { id: 'sec-summary', title: t.professionalSummary },
    { id: 'sec-experience', title: t.workExperience },
    { id: 'sec-education', title: t.education },
    { id: 'sec-skills', title: t.skills },
    { id: 'sec-projects', title: t.projects },
    { id: 'sec-certifications', title: t.certifications },
    { id: 'sec-languages', title: t.languages },
  ];
  const title = (id: string) => sections.find((section) => section.id === id)!.title;

  const pageLabel = preview ? `${preview.pages} ${preview.pages === 1 ? t.page : t.pages}` : '';
  const previewNote =
    status === 'rendering'
      ? t.rendering
      : stale
        ? status === 'failed'
          ? t.previewOutdatedFailed
          : t.previewOutdated
        : status === 'failed'
          ? t.statusFailed
          : preview
            ? t.upToDate
            : '';

  return (
    <div className="app">
      <header className="topbar">
        <button type="button" className="ghost back" onClick={() => void requestClose('library')}>
          <span aria-hidden="true">←</span> {t.library}
        </button>
        <span className="divider" aria-hidden="true" />
        <h1 className="doc-title">{projectLabel({ title: doc.title, name: doc.cv.name }, t.untitledCv)}</h1>
        <span className={`save ${saveState}`} role="status" title={t.editorTagline}>
          {t[SAVE_KEY[saveState]]}
        </span>
        {saveState === 'failed' && (
          <button type="button" onClick={() => void persist()}>
            {t.retrySave}
          </button>
        )}
        <span className="spacer" />
        <button type="button" className="ghost" onClick={() => void exportBackup()}>
          {t.exportBackup}
        </button>
        {languageChoice}
        <button type="button" className="primary" onClick={() => void exportPdf()}>
          {t.exportPdf}
        </button>
      </header>

      {(recovery || backupStatus || (!closePrompt && (saveError || error))) && (
        <div className="banners">
          {recovery && (
            <p className="notice" role="status">
              {recovery}
            </p>
          )}
          {backupStatus && (
            <p className="success" role="status">
              {t.backupExported}: {backupStatus}
            </p>
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
        </div>
      )}

      {narrow && (
        <div className="segmented panes" role="group" aria-label={t.panesAria}>
          <button type="button" aria-pressed={pane === 'form'} onClick={() => setPane('form')}>
            {t.formTab}
          </button>
          <button type="button" aria-pressed={pane === 'preview'} onClick={() => setPane('preview')}>
            {t.previewAria}
          </button>
        </div>
      )}

      <main className="workspace">
        <section className="editor" aria-label={t.editorAria} hidden={narrow && pane !== 'form'}>
          <nav className="jump" aria-label={t.sectionsAria}>
            {sections.map((section) => (
              <a key={section.id} href={`#${section.id}`}>
                {section.title}
              </a>
            ))}
          </nav>

          <div className="cards">
            <label className="doc-name">
              {t.projectNameOptional}
              <input
                value={doc.title}
                placeholder={doc.cv.name.trim() || t.untitledCv}
                onChange={(e) => edit((project) => ({ ...project, title: e.target.value }))}
              />
            </label>

            <Card id="sec-contact" title={title('sec-contact')}>
              <div className="grid2">
                <label>
                  {t.fullName}
                  <input value={doc.cv.name} onChange={(e) => setField('name', e.target.value)} />
                </label>
                <label>
                  {t.headlineOptional}
                  <input
                    value={doc.cv.headline}
                    placeholder={t.headlineExample}
                    onChange={(e) => setField('headline', e.target.value)}
                  />
                </label>
              </div>
              <div className="grid2">
                <label>
                  {t.email}
                  <input type="email" value={doc.cv.email} onChange={(e) => setField('email', e.target.value)} />
                </label>
                <label>
                  {t.phoneOptional}
                  <input type="tel" value={doc.cv.phone} onChange={(e) => setField('phone', e.target.value)} />
                </label>
              </div>
              <label>
                {t.locationOptional}
                <input value={doc.cv.location} onChange={(e) => setField('location', e.target.value)} />
              </label>
              <h3 className="sub">{t.linksOptional}</h3>
              <LinksSection items={doc.cv.links} onChange={(links) => editCv((cv) => ({ ...cv, links }))} />
            </Card>

            <Card id="sec-summary" title={title('sec-summary')}>
              <label>
                {t.summary}
                <textarea rows={5} value={doc.cv.summary} onChange={(e) => setField('summary', e.target.value)} />
              </label>
            </Card>

            <Card id="sec-experience" title={title('sec-experience')}>
              <ExperienceSection
                items={doc.cv.experience}
                onChange={(experience) => editCv((cv) => ({ ...cv, experience }))}
              />
            </Card>

            <Card id="sec-education" title={title('sec-education')}>
              <EducationSection
                items={doc.cv.education}
                onChange={(education) => editCv((cv) => ({ ...cv, education }))}
              />
            </Card>

            <Card id="sec-skills" title={title('sec-skills')}>
              <SkillsSection items={doc.cv.skills} onChange={(skills) => editCv((cv) => ({ ...cv, skills }))} />
            </Card>

            <Card id="sec-projects" title={title('sec-projects')}>
              <ProjectsSection
                items={doc.cv.projects}
                onChange={(projects) => editCv((cv) => ({ ...cv, projects }))}
              />
            </Card>

            <Card id="sec-certifications" title={title('sec-certifications')}>
              <CertificationsSection
                items={doc.cv.certifications}
                onChange={(certifications) => editCv((cv) => ({ ...cv, certifications }))}
              />
            </Card>

            <Card id="sec-languages" title={title('sec-languages')}>
              <LanguagesSection
                items={doc.cv.languages}
                onChange={(languages) => editCv((cv) => ({ ...cv, languages }))}
              />
            </Card>
          </div>
        </section>

        <section className="preview" aria-label={t.previewAria} hidden={narrow && pane !== 'preview'}>
          <div className="toolbar">
            <fieldset className="segmented structure">
              <legend className="sr-only">{t.contentStructure}</legend>
              {STRUCTURES.map((value) => (
                <label key={value} title={t.structureHint}>
                  <input
                    type="radio"
                    name="structure"
                    className="sr-only"
                    checked={doc.structure === value}
                    onChange={() => edit((project) => ({ ...project, structure: value }))}
                  />
                  {t[STRUCTURE_KEY[value]]}
                </label>
              ))}
            </fieldset>
            <label className="inline" title={t.contentLanguageHint}>
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
          </div>

          <div className="statusline">
            <span className={`dot ${stale || status === 'failed' ? 'warn' : status === 'rendering' ? 'busy' : 'ok'}`} aria-hidden="true" />
            {/* Prior bytes stay visible while regenerating, explicitly outdated. */}
            <span role="status">{previewNote}</span>
            {pageLabel && <span className="muted">· {pageLabel}</span>}
            <span className="spacer" />
            <button type="button" className="ghost small" onClick={() => void render(doc)} disabled={status === 'rendering'}>
              {t.renderPreview}
            </button>
          </div>

          {renderError && (
            <p className="error" role="alert">
              {renderError}
            </p>
          )}

          {exported && (
            <div className={exported.changed || !exported.matches ? 'notice' : 'success'} role="status">
              <p>
                {t.exportedTo} {exported.path}
              </p>
              {/* Announced, not just appended to the export line: the file is stale. */}
              {exported.changed && <p>{t.exportChanged}</p>}
              {!exported.matches && <p>{t.differentBytes}</p>}
              {exported.warnings.length > 0 && (
                <ul>
                  {exported.warnings.map((warning) => (
                    <li key={warning}>{warning}</li>
                  ))}
                </ul>
              )}
            </div>
          )}

          <div ref={host} className="pages" />

          <details className="tech">
            <summary>{t.renderDetails}</summary>
            <dl>
              <dt>{t.save}</dt>
              <dd>{t[SAVE_KEY[saveState]]}</dd>
              <dt>{t.render}</dt>
              <dd>{t[STATUS_KEY[status]]}</dd>
              <dt>{t.preview}</dt>
              <dd>
                {preview
                  ? `${pageLabel}, ${t.revision} ${preview.revision}${stale ? ` ${t.outdated}` : ''}`
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
          </details>
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
              <p className="success" role="status">
                {t.backupExported}: {backupStatus}
              </p>
            )}
            <div className="actions end">
              <button type="button" onClick={() => setClosePrompt(null)}>
                {t.cancel}
              </button>
              <button type="button" className="ghost danger" onClick={() => void finishClose(closePrompt)}>
                {t.discardAndClose}
              </button>
              <button type="button" onClick={() => void exportBackup()}>
                {t.exportBackup}
              </button>
              <button type="button" className="primary" autoFocus onClick={() => void requestClose(closePrompt)}>
                {t.retrySave}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
