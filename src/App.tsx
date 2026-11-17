import { useEffect, useState } from 'react';
import type { Lang, Project } from './cv';
import Editor from './Editor';
import Library from './Library';
import { LANGS, LangContext, STRINGS, saveLang, storedLang } from './i18n';

/** Interface language only: language names stay native so either reader finds theirs. */
function LanguageChoice({ value, onChange }: { value: Lang; onChange: (next: Lang) => void }) {
  return (
    <label className="inline lang">
      {STRINGS[value].interfaceShort}
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

/** Library or one open CV. The editor is keyed by project, so each open starts clean. */
export default function App() {
  const [lang, setLang] = useState<Lang>(storedLang);
  const [open, setOpen] = useState<{ project: Project; notice?: string } | null>(null);

  // Interface language is an application preference: it never touches CV data.
  useEffect(() => {
    saveLang(lang);
    document.documentElement.lang = lang === 'indonesian' ? 'id' : 'en';
  }, [lang]);

  const languageChoice = <LanguageChoice value={lang} onChange={setLang} />;

  return (
    <LangContext.Provider value={{ lang, t: STRINGS[lang] }}>
      {open ? (
        <Editor
          key={open.project.id}
          initial={open.project}
          recoveryNotice={open.notice}
          languageChoice={languageChoice}
          onLeave={() => setOpen(null)}
        />
      ) : (
        <Library languageChoice={languageChoice} onOpen={(project, notice) => setOpen({ project, notice })} />
      )}
    </LangContext.Provider>
  );
}
