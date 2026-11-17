import type { ReactNode } from 'react';
import {
  emptyCertification,
  emptyDate,
  emptyEducation,
  emptyExperience,
  emptyLanguage,
  emptyProject,
  emptySkill,
  moved,
  type Certification,
  type CvDate,
  type Education,
  type Experience,
  type Language,
  type Link,
  type ProjectEntry,
  type SkillGroup,
} from './cv';
import { MONTHS, useT } from './i18n';

const patched = <T,>(items: T[], index: number, patch: Partial<T>): T[] =>
  items.map((item, i) => (i === index ? { ...item, ...patch } : item));

const without = <T,>(items: T[], index: number): T[] => items.filter((_, i) => i !== index);

/** Up/down/remove controls: reordering never requires a mouse drag. */
function ItemControls({
  label,
  index,
  count,
  onMove,
  onRemove,
}: {
  label: string;
  index: number;
  count: number;
  onMove: (from: number, to: number) => void;
  onRemove: (index: number) => void;
}) {
  const { t } = useT();
  return (
    <span className="controls">
      <button
        type="button"
        className="icon"
        aria-label={`${t.move} ${label} ${t.up}`}
        disabled={index === 0}
        onClick={() => onMove(index, index - 1)}
      >
        ↑
      </button>
      <button
        type="button"
        className="icon"
        aria-label={`${t.move} ${label} ${t.down}`}
        disabled={index === count - 1}
        onClick={() => onMove(index, index + 1)}
      >
        ↓
      </button>
      <button
        type="button"
        className="icon danger"
        aria-label={`${t.remove} ${label}`}
        title={t.remove}
        onClick={() => onRemove(index)}
      >
        ×
      </button>
    </span>
  );
}

/**
 * Every repeatable CV entry: a labelled group with reorder/remove controls and
 * an add button. A section supplies only its fields and how to name an entry.
 */
function EntryList<T>({
  items,
  onChange,
  label,
  addLabel,
  create,
  name,
  children,
}: {
  items: T[];
  onChange: (next: T[]) => void;
  /** "Work experience": names the entry group and its controls. */
  label: string;
  addLabel: string;
  create: () => T;
  /** What the user typed that identifies an entry, shown instead of "Work experience 2". */
  name: (item: T) => string;
  children: (item: T, patch: (change: Partial<T>) => void, scope: string) => ReactNode;
}) {
  return (
    <>
      {items.map((item, index) => {
        const scope = `${label.toLowerCase()} ${index + 1}`;
        return (
          <fieldset className="entry" key={index}>
            {/* The legend stays plain text: buttons inside it become part of the
                group's accessible name ("Work experience 1 Move … Remove"). */}
            <legend>{name(item).trim() || `${label} ${index + 1}`}</legend>
            <ItemControls
              label={scope}
              index={index}
              count={items.length}
              onMove={(from, to) => onChange(moved(items, from, to))}
              onRemove={(position) => onChange(without(items, position))}
            />
            {children(item, (change) => onChange(patched(items, index, change)), scope)}
          </fieldset>
        );
      })}
      <button type="button" className="add" onClick={() => onChange([...items, create()])}>
        <span aria-hidden="true">+</span> {addLabel}
      </button>
    </>
  );
}

/** Year is optional; a month can only be selected when a year exists. */
function DateField({
  label,
  value,
  onChange,
  disabled = false,
}: {
  label: string;
  value: CvDate;
  onChange: (next: CvDate) => void;
  disabled?: boolean;
}) {
  const { lang, t } = useT();
  const setYear = (text: string) => {
    const digits = text.replace(/\D/g, '').slice(0, 4);
    const year = digits === '' ? null : Number(digits);
    onChange({ year, month: year === null ? null : value.month });
  };

  return (
    <div className="date-field">
      <span className="field-label">{label}</span>
      <div className="date-inputs">
        <select
          aria-label={`${label} ${t.month}`}
          value={value.month ?? ''}
          disabled={disabled || value.year === null}
          onChange={(e) => onChange({ year: value.year, month: e.target.value === '' ? null : Number(e.target.value) })}
        >
          <option value="">{t.noMonth}</option>
          {MONTHS[lang].map((month, index) => (
            <option key={month} value={index + 1}>
              {month}
            </option>
          ))}
        </select>
        <input
          aria-label={`${label} ${t.year}`}
          inputMode="numeric"
          placeholder={t.yearExample}
          value={value.year ?? ''}
          disabled={disabled}
          onChange={(e) => setYear(e.target.value)}
        />
      </div>
    </div>
  );
}

/** Start, end, and Present for dated history: an ongoing entry has no end date. */
function DateRange({
  start,
  end,
  patch,
}: {
  start: CvDate;
  end: CvDate | null;
  patch: (change: { start?: CvDate; end?: CvDate | null }) => void;
}) {
  const { t } = useT();
  return (
    <>
      <div className="grid2">
        <DateField label={t.start} value={start} onChange={(next) => patch({ start: next })} />
        <DateField
          label={t.end}
          value={end ?? emptyDate()}
          disabled={end === null}
          onChange={(next) => patch({ end: next })}
        />
      </div>
      <label className="check">
        <input
          type="checkbox"
          checked={end === null}
          onChange={(e) => patch({ end: e.target.checked ? null : emptyDate() })}
        />
        {t.ongoing}
      </label>
    </>
  );
}

function Bullets({ scope, bullets, onChange }: { scope: string; bullets: string[]; onChange: (next: string[]) => void }) {
  const { t } = useT();
  return (
    <div className="bullets">
      <span className="field-label">{t.achievementBullets}</span>
      {bullets.map((bullet, index) => (
        <div className="bullet-row" key={index}>
          <span className="bullet-mark" aria-hidden="true">
            •
          </span>
          <input
            aria-label={`${scope} ${t.bullet} ${index + 1}`}
            value={bullet}
            onChange={(e) => onChange(bullets.map((text, i) => (i === index ? e.target.value : text)))}
          />
          <ItemControls
            label={`${scope} ${t.bullet} ${index + 1}`}
            index={index}
            count={bullets.length}
            onMove={(from, to) => onChange(moved(bullets, from, to))}
            onRemove={(position) => onChange(without(bullets, position))}
          />
        </div>
      ))}
      <button
        type="button"
        className="ghost small"
        aria-label={`${t.addBulletTo} ${scope}`}
        onClick={() => onChange([...bullets, ''])}
      >
        <span aria-hidden="true">+</span> {t.addBullet}
      </button>
    </div>
  );
}

/** A labelled single-line text field. */
function Text({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (next: string) => void;
  placeholder?: string;
}) {
  return (
    <label>
      {label}
      <input value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} />
    </label>
  );
}

const joined = (...parts: string[]) => parts.filter((part) => part.trim()).join(' · ');

export function ExperienceSection({ items, onChange }: { items: Experience[]; onChange: (next: Experience[]) => void }) {
  const { t } = useT();
  return (
    <EntryList
      items={items}
      onChange={onChange}
      label={t.workExperience}
      addLabel={t.addWorkExperience}
      create={emptyExperience}
      name={(entry) => joined(entry.title, entry.company)}
    >
      {(entry, patch, scope) => (
        <>
          <div className="grid2">
            <Text label={t.jobTitle} value={entry.title} onChange={(title) => patch({ title })} />
            <Text label={t.employer} value={entry.company} onChange={(company) => patch({ company })} />
          </div>
          <Text label={t.location} value={entry.location} onChange={(location) => patch({ location })} />
          <DateRange start={entry.start} end={entry.end} patch={patch} />
          <Bullets scope={scope} bullets={entry.bullets} onChange={(bullets) => patch({ bullets })} />
        </>
      )}
    </EntryList>
  );
}

export function EducationSection({ items, onChange }: { items: Education[]; onChange: (next: Education[]) => void }) {
  const { t } = useT();
  return (
    <EntryList
      items={items}
      onChange={onChange}
      label={t.education}
      addLabel={t.addEducation}
      create={emptyEducation}
      name={(entry) => joined(entry.degree, entry.school)}
    >
      {(entry, patch, scope) => (
        <>
          <Text label={t.qualification} value={entry.degree} onChange={(degree) => patch({ degree })} />
          <div className="grid2">
            <Text label={t.school} value={entry.school} onChange={(school) => patch({ school })} />
            <Text label={t.location} value={entry.location} onChange={(location) => patch({ location })} />
          </div>
          <DateRange start={entry.start} end={entry.end} patch={patch} />
          <Bullets scope={scope} bullets={entry.bullets} onChange={(bullets) => patch({ bullets })} />
        </>
      )}
    </EntryList>
  );
}

export function SkillsSection({ items, onChange }: { items: SkillGroup[]; onChange: (next: SkillGroup[]) => void }) {
  const { t } = useT();
  return (
    <EntryList
      items={items}
      onChange={onChange}
      label={t.skillGroup}
      addLabel={t.addSkillGroup}
      create={emptySkill}
      name={(skill) => skill.name}
    >
      {(skill, patch, scope) => (
        <>
          <Text label={t.skillGroup} value={skill.name} onChange={(name) => patch({ name })} />
          <Bullets scope={scope} bullets={skill.bullets} onChange={(bullets) => patch({ bullets })} />
        </>
      )}
    </EntryList>
  );
}

export function ProjectsSection({ items, onChange }: { items: ProjectEntry[]; onChange: (next: ProjectEntry[]) => void }) {
  const { t } = useT();
  return (
    <EntryList
      items={items}
      onChange={onChange}
      label={t.project}
      addLabel={t.addProject}
      create={emptyProject}
      name={(project) => project.name}
    >
      {(project, patch, scope) => (
        <>
          <div className="grid2">
            <Text label={t.projectName} value={project.name} onChange={(name) => patch({ name })} />
            <Text label={t.contextOptional} value={project.detail} onChange={(detail) => patch({ detail })} />
          </div>
          <Bullets scope={scope} bullets={project.bullets} onChange={(bullets) => patch({ bullets })} />
        </>
      )}
    </EntryList>
  );
}

export function CertificationsSection({
  items,
  onChange,
}: {
  items: Certification[];
  onChange: (next: Certification[]) => void;
}) {
  const { t } = useT();
  return (
    <EntryList
      items={items}
      onChange={onChange}
      label={t.certification}
      addLabel={t.addCertification}
      create={emptyCertification}
      name={(certification) => certification.name}
    >
      {(certification, patch) => (
        <>
          <div className="grid2">
            <Text label={t.certification} value={certification.name} onChange={(name) => patch({ name })} />
            <Text label={t.issuerOptional} value={certification.issuer} onChange={(issuer) => patch({ issuer })} />
          </div>
          <DateField label={t.date} value={certification.date} onChange={(date) => patch({ date })} />
        </>
      )}
    </EntryList>
  );
}

export function LanguagesSection({ items, onChange }: { items: Language[]; onChange: (next: Language[]) => void }) {
  const { t } = useT();
  return (
    <EntryList
      items={items}
      onChange={onChange}
      label={t.language}
      addLabel={t.addLanguage}
      create={emptyLanguage}
      name={(language) => language.name}
    >
      {(language, patch) => (
        <div className="grid2">
          <Text label={t.language} value={language.name} onChange={(name) => patch({ name })} />
          <Text label={t.levelOptional} value={language.level} onChange={(level) => patch({ level })} />
        </div>
      )}
    </EntryList>
  );
}

export function LinksSection({ items, onChange }: { items: Link[]; onChange: (next: Link[]) => void }) {
  const { t } = useT();
  return (
    <EntryList
      items={items}
      onChange={onChange}
      label={t.link}
      addLabel={t.addLink}
      create={() => ({ label: '', url: '' })}
      name={(link) => link.label}
    >
      {(link, patch) => (
        <div className="grid2">
          <Text label={t.linkLabelOptional} value={link.label} onChange={(label) => patch({ label })} />
          <Text
            label={t.webAddress}
            value={link.url}
            placeholder="https://example.com/portfolio"
            onChange={(url) => patch({ url })}
          />
        </div>
      )}
    </EntryList>
  );
}
