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
      <button type="button" aria-label={`${t.move} ${label} ${t.up}`} disabled={index === 0} onClick={() => onMove(index, index - 1)}>
        ↑
      </button>
      <button type="button" aria-label={`${t.move} ${label} ${t.down}`} disabled={index === count - 1} onClick={() => onMove(index, index + 1)}>
        ↓
      </button>
      <button type="button" aria-label={`${t.remove} ${label}`} onClick={() => onRemove(index)}>
        {t.remove}
      </button>
    </span>
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
    <div className="row">
      <label>
        {label} {t.month}
        <select
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
      </label>
      <label>
        {label} {t.year}
        <input
          inputMode="numeric"
          placeholder={t.yearExample}
          value={value.year ?? ''}
          disabled={disabled}
          onChange={(e) => setYear(e.target.value)}
        />
      </label>
    </div>
  );
}

function Bullets({
  label,
  scope,
  bullets,
  onChange,
}: {
  label: string;
  scope: string;
  bullets: string[];
  onChange: (next: string[]) => void;
}) {
  const { t } = useT();
  return (
    <div className="bullets">
      <span className="field-label">{label}</span>
      {bullets.map((bullet, index) => (
        <div className="bullet-row" key={index}>
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
      <button type="button" aria-label={`${t.addBulletTo} ${scope}`} onClick={() => onChange([...bullets, ''])}>
        {t.addBullet}
      </button>
    </div>
  );
}

function EntryLegend({ index, count, label, onMove, onRemove }: { index: number; count: number; label: string; onMove: (from: number, to: number) => void; onRemove: (index: number) => void }) {
  // The legend stays plain text: buttons inside it become part of the group's
  // accessible name ("Work experience 1 Move … Remove").
  return (
    <>
      <legend>
        {label} {index + 1}
      </legend>
      <div className="entry-head">
        <ItemControls label={`${label.toLowerCase()} ${index + 1}`} index={index} count={count} onMove={onMove} onRemove={onRemove} />
      </div>
    </>
  );
}

export function ExperienceSection({ items, onChange }: { items: Experience[]; onChange: (next: Experience[]) => void }) {
  const { t } = useT();
  const scope = t.workExperience.toLowerCase();
  return (
    <>
      {items.map((entry, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label={t.workExperience}
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            {t.jobTitle}
            <input value={entry.title} onChange={(e) => onChange(patched(items, index, { title: e.target.value }))} />
          </label>
          <label>
            {t.employer}
            <input value={entry.company} onChange={(e) => onChange(patched(items, index, { company: e.target.value }))} />
          </label>
          <label>
            {t.location}
            <input value={entry.location} onChange={(e) => onChange(patched(items, index, { location: e.target.value }))} />
          </label>
          <DateField label={t.start} value={entry.start} onChange={(start) => onChange(patched(items, index, { start }))} />
          <DateField
            label={t.end}
            value={entry.end ?? emptyDate()}
            disabled={entry.end === null}
            onChange={(end) => onChange(patched(items, index, { end }))}
          />
          <label className="check">
            <input
              type="checkbox"
              checked={entry.end === null}
              onChange={(e) => onChange(patched(items, index, { end: e.target.checked ? null : emptyDate() }))}
            />
            {t.ongoing}
          </label>
          <Bullets
            label={t.achievementBullets}
            scope={`${scope} ${index + 1}`}
            bullets={entry.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyExperience()])}>
        {t.addWorkExperience}
      </button>
    </>
  );
}

export function EducationSection({ items, onChange }: { items: Education[]; onChange: (next: Education[]) => void }) {
  const { t } = useT();
  const scope = t.education.toLowerCase();
  return (
    <>
      {items.map((entry, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label={t.education}
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            {t.qualification}
            <input value={entry.degree} onChange={(e) => onChange(patched(items, index, { degree: e.target.value }))} />
          </label>
          <label>
            {t.school}
            <input value={entry.school} onChange={(e) => onChange(patched(items, index, { school: e.target.value }))} />
          </label>
          <label>
            {t.location}
            <input value={entry.location} onChange={(e) => onChange(patched(items, index, { location: e.target.value }))} />
          </label>
          <DateField label={t.start} value={entry.start} onChange={(start) => onChange(patched(items, index, { start }))} />
          <DateField
            label={t.end}
            value={entry.end ?? emptyDate()}
            disabled={entry.end === null}
            onChange={(end) => onChange(patched(items, index, { end }))}
          />
          <label className="check">
            <input
              type="checkbox"
              checked={entry.end === null}
              onChange={(e) => onChange(patched(items, index, { end: e.target.checked ? null : emptyDate() }))}
            />
            {t.ongoing}
          </label>
          <Bullets
            label={t.achievementBullets}
            scope={`${scope} ${index + 1}`}
            bullets={entry.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyEducation()])}>
        {t.addEducation}
      </button>
    </>
  );
}

export function SkillsSection({ items, onChange }: { items: SkillGroup[]; onChange: (next: SkillGroup[]) => void }) {
  const { t } = useT();
  const scope = t.skillGroup.toLowerCase();
  return (
    <>
      {items.map((skill, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label={t.skillGroup}
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            {t.skillGroup}
            <input value={skill.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <Bullets
            label={t.achievementBullets}
            scope={`${scope} ${index + 1}`}
            bullets={skill.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptySkill()])}>
        {t.addSkillGroup}
      </button>
    </>
  );
}

export function ProjectsSection({ items, onChange }: { items: ProjectEntry[]; onChange: (next: ProjectEntry[]) => void }) {
  const { t } = useT();
  const scope = t.project.toLowerCase();
  return (
    <>
      {items.map((project, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label={t.project}
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            {t.projectName}
            <input value={project.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <label>
            {t.contextOptional}
            <input value={project.detail} onChange={(e) => onChange(patched(items, index, { detail: e.target.value }))} />
          </label>
          <Bullets
            label={t.achievementBullets}
            scope={`${scope} ${index + 1}`}
            bullets={project.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyProject()])}>
        {t.addProject}
      </button>
    </>
  );
}

export function CertificationsSection({ items, onChange }: { items: Certification[]; onChange: (next: Certification[]) => void }) {
  const { t } = useT();
  const scope = t.certification.toLowerCase();
  return (
    <>
      {items.map((certification, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label={t.certification}
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            {t.certification}
            <input value={certification.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <label>
            {t.issuerOptional}
            <input value={certification.issuer} onChange={(e) => onChange(patched(items, index, { issuer: e.target.value }))} />
          </label>
          <DateField label={t.date} value={certification.date} onChange={(date) => onChange(patched(items, index, { date }))} />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyCertification()])}>
        {t.addCertification}
      </button>
    </>
  );
}

export function LanguagesSection({ items, onChange }: { items: Language[]; onChange: (next: Language[]) => void }) {
  const { t } = useT();
  const scope = t.language.toLowerCase();
  return (
    <>
      {items.map((language, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label={t.language}
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            {t.language}
            <input value={language.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <label>
            {t.levelOptional}
            <input value={language.level} onChange={(e) => onChange(patched(items, index, { level: e.target.value }))} />
          </label>
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyLanguage()])}>
        {t.addLanguage}
      </button>
    </>
  );
}

export function LinksSection({ items, onChange }: { items: Link[]; onChange: (next: Link[]) => void }) {
  const { t } = useT();
  const scope = t.link.toLowerCase();
  return (
    <>
      {items.map((link, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label={t.link}
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            {t.linkLabelOptional}
            <input value={link.label} onChange={(e) => onChange(patched(items, index, { label: e.target.value }))} />
          </label>
          <label>
            {t.webAddress}
            <input
              value={link.url}
              placeholder="https://example.com/portfolio"
              onChange={(e) => onChange(patched(items, index, { url: e.target.value }))}
            />
          </label>
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, { label: '', url: '' }])}>
        {t.addLink}
      </button>
    </>
  );
}
