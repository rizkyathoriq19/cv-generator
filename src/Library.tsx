import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { projectLabel, type Opened, type Project, type ProjectSummary } from './cv';
import { LANGS, STRUCTURE_KEY, useT } from './i18n';

/**
 * The local library: active CVs and Trash. Opening, creating, duplicating, or
 * importing hands the project to the editor; this view keeps nothing open.
 */
export default function Library({
  languageChoice,
  onOpen,
}: {
  languageChoice: ReactNode;
  onOpen: (project: Project, recoveryNotice?: string) => void;
}) {
  const { lang, t } = useT();
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [trash, setTrash] = useState<ProjectSummary[]>([]);
  const [showTrash, setShowTrash] = useState(false);
  const [error, setError] = useState('');
  // Permanent deletion waits for an explicit confirmation, never a row click.
  const [pendingDelete, setPendingDelete] = useState<ProjectSummary | null>(null);
  // A permanent-deletion confirmation traps Tab between its two buttons.
  const cancelDelete = useRef<HTMLButtonElement>(null);
  const confirmDelete = useRef<HTMLButtonElement>(null);
  const dateLocale = lang === 'indonesian' ? 'id-ID' : 'en-US';

  const refresh = useCallback(async () => {
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
    void refresh();
  }, [refresh]);

  /** Every library action reports its own failure and leaves the list current. */
  const run = async (action: () => Promise<void>) => {
    setError('');
    try {
      await action();
    } catch (e) {
      setError(String(e));
    }
  };

  const createProject = () => run(async () => onOpen(await invoke<Project>('create_project')));

  /** Duplication copies content, structure, and content language into a new CV. */
  const duplicateProject = (id: string) => run(async () => onOpen(await invoke<Project>('duplicate_project', { id })));

  /** Import always creates a new CV; the backup file and existing CVs are never modified. */
  const importBackup = () =>
    run(async () => {
      const path = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: t.backupFile, extensions: ['json'] }],
      });
      if (!path) return;
      onOpen(await invoke<Project>('import_backup', { path, interfaceLanguage: lang }));
    });

  // A damaged file is recovered from its previous revision with a notice;
  // without one, the open reports the damage instead of blanking the CV.
  const openProject = (id: string) =>
    run(async () => {
      const opened = await invoke<Opened>('open_project', { id, interfaceLanguage: lang });
      onOpen(opened.project, opened.recovery?.notice);
    });

  const moveToTrash = (id: string) =>
    run(async () => {
      await invoke('delete_project', { id, interfaceLanguage: lang });
      await refresh();
    });

  const restoreProject = (id: string) =>
    run(async () => {
      await invoke('restore_project', { id, interfaceLanguage: lang });
      await refresh();
    });

  /** Only the confirmation dialog calls this: permanent deletion is never implicit. */
  const deletePermanently = (id: string) =>
    run(async () => {
      await invoke('delete_project_permanently', { id, interfaceLanguage: lang });
      setPendingDelete(null);
      await refresh();
    });

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

  /** "English · Combination · Updated 17 Nov 2026, 13:40" — what tells copies apart. */
  const meta = (project: ProjectSummary) => {
    const parts: string[] = [];
    const language = LANGS.find((option) => option.value === project.content_language);
    if (language) parts.push(language.label);
    if (project.structure) parts.push(t[STRUCTURE_KEY[project.structure]]);
    if (project.updated_at > 0) {
      parts.push(
        `${t.updated} ${new Date(project.updated_at * 1000).toLocaleString(dateLocale, {
          dateStyle: 'medium',
          timeStyle: 'short',
        })}`,
      );
    }
    return parts.join(' · ');
  };

  const rowText = (project: ProjectSummary, label: string) => (
    <>
      <span className="row-title">
        {label}
        {project.damaged && <span className="tag danger">{t.damaged}</span>}
      </span>
      <span className="row-meta">{meta(project)}</span>
    </>
  );

  const rows = showTrash ? trash : projects;

  return (
    <div className="app">
      <header className="topbar">
        <span className="brand">CV Generator</span>
        <span className="spacer" />
        {languageChoice}
      </header>

      <main className="library">
        <div className="page-head">
          <div>
            <h1>{t.yourCvs}</h1>
            <p className="muted">{t.libraryTagline}</p>
          </div>
          <div className="actions">
            <button type="button" onClick={() => void importBackup()}>
              {t.importBackup}
            </button>
            <button type="button" className="primary" onClick={() => void createProject()}>
              {t.newCv}
            </button>
          </div>
        </div>

        <div className="library-views">
          <div className="segmented" role="group" aria-label={t.libraryViews}>
            <button type="button" aria-pressed={!showTrash} onClick={() => setShowTrash(false)}>
              {t.cvsTab} <span className="count">{projects.length}</span>
            </button>
            <button
              type="button"
              aria-pressed={showTrash}
              onClick={() => {
                setError('');
                setShowTrash(true);
              }}
            >
              {t.trash} <span className="count">{trash.length}</span>
            </button>
          </div>
        </div>

        {!pendingDelete && error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}

        {rows.length === 0 ? (
          <div className="empty">
            <p>{showTrash ? t.trashEmpty : t.noCvs}</p>
            {!showTrash && (
              <button type="button" className="primary" onClick={() => void createProject()}>
                {t.newCv}
              </button>
            )}
          </div>
        ) : (
          <ul className="list">
            {rows.map((project) => {
              const label = projectLabel(project, t.untitledCv);
              return showTrash ? (
                <li key={project.id} className="project-row">
                  <div className="row-main">{rowText(project, label)}</div>
                  <div className="row-actions">
                    <button type="button" aria-label={`${t.restore} ${label}`} onClick={() => void restoreProject(project.id)}>
                      {t.restore}
                    </button>
                    <button
                      type="button"
                      className="ghost danger"
                      aria-label={`${t.deletePermanently} ${label}`}
                      onClick={() => {
                        setError('');
                        setPendingDelete(project);
                      }}
                    >
                      {t.deletePermanently}
                    </button>
                  </div>
                </li>
              ) : (
                <li key={project.id} className="project-row">
                  <button
                    type="button"
                    className="row-main"
                    title={project.damaged ? t.damagedHint : undefined}
                    onClick={() => void openProject(project.id)}
                  >
                    {rowText(project, label)}
                  </button>
                  <div className="row-actions">
                    <button
                      type="button"
                      className="ghost"
                      aria-label={`${t.duplicate} ${label}`}
                      onClick={() => void duplicateProject(project.id)}
                    >
                      {t.duplicate}
                    </button>
                    <button
                      type="button"
                      className="ghost"
                      aria-label={`${t.moveToTrash} ${label}`}
                      onClick={() => void moveToTrash(project.id)}
                    >
                      {t.moveToTrash}
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}

        <p className="muted note">{showTrash ? t.trashNote : t.backupNote}</p>
      </main>

      {pendingDelete && (
        <div className="overlay" role="dialog" aria-modal="true" aria-labelledby="delete-title">
          <div className="dialog">
            <h2 id="delete-title">{t.deleteTitle}</h2>
            <p>
              <strong>{projectLabel(pendingDelete, t.untitledCv)}</strong>
            </p>
            <p className="muted">{t.deleteBody}</p>
            {error && (
              <p className="error" role="alert">
                {error}
              </p>
            )}
            <div className="actions end">
              <button type="button" ref={cancelDelete} autoFocus onClick={() => setPendingDelete(null)}>
                {t.cancel}
              </button>
              <button
                type="button"
                className="danger-solid"
                ref={confirmDelete}
                onClick={() => void deletePermanently(pendingDelete.id)}
              >
                {t.deletePermanently}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
