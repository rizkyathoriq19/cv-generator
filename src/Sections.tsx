import {
  MONTHS,
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
  return (
    <span className="controls">
      <button type="button" aria-label={`Move ${label} up`} disabled={index === 0} onClick={() => onMove(index, index - 1)}>
        ↑
      </button>
      <button type="button" aria-label={`Move ${label} down`} disabled={index === count - 1} onClick={() => onMove(index, index + 1)}>
        ↓
      </button>
      <button type="button" aria-label={`Remove ${label}`} onClick={() => onRemove(index)}>
        Remove
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
  const setYear = (text: string) => {
    const digits = text.replace(/\D/g, '').slice(0, 4);
    const year = digits === '' ? null : Number(digits);
    onChange({ year, month: year === null ? null : value.month });
  };

  return (
    <div className="row">
      <label>
        {label} month
        <select
          value={value.month ?? ''}
          disabled={disabled || value.year === null}
          onChange={(e) => onChange({ year: value.year, month: e.target.value === '' ? null : Number(e.target.value) })}
        >
          <option value="">No month</option>
          {MONTHS.map((month, index) => (
            <option key={month} value={index + 1}>
              {month}
            </option>
          ))}
        </select>
      </label>
      <label>
        {label} year
        <input
          inputMode="numeric"
          placeholder="e.g. 2020"
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
  return (
    <div className="bullets">
      <span className="field-label">{label}</span>
      {bullets.map((bullet, index) => (
        <div className="bullet-row" key={index}>
          <input
            aria-label={`${scope} bullet ${index + 1}`}
            value={bullet}
            onChange={(e) => onChange(bullets.map((text, i) => (i === index ? e.target.value : text)))}
          />
          <ItemControls
            label={`${scope} bullet ${index + 1}`}
            index={index}
            count={bullets.length}
            onMove={(from, to) => onChange(moved(bullets, from, to))}
            onRemove={(position) => onChange(without(bullets, position))}
          />
        </div>
      ))}
      <button type="button" aria-label={`Add bullet to ${scope}`} onClick={() => onChange([...bullets, ''])}>
        Add bullet
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
  return (
    <>
      {items.map((entry, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label="Work experience"
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            Job title
            <input value={entry.title} onChange={(e) => onChange(patched(items, index, { title: e.target.value }))} />
          </label>
          <label>
            Employer
            <input value={entry.company} onChange={(e) => onChange(patched(items, index, { company: e.target.value }))} />
          </label>
          <label>
            Location
            <input value={entry.location} onChange={(e) => onChange(patched(items, index, { location: e.target.value }))} />
          </label>
          <DateField label="Start" value={entry.start} onChange={(start) => onChange(patched(items, index, { start }))} />
          <DateField
            label="End"
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
            Ongoing (Present)
          </label>
          <Bullets
            label="Achievement bullets"
            scope={`work experience ${index + 1}`}
            bullets={entry.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyExperience()])}>
        Add work experience
      </button>
    </>
  );
}

export function EducationSection({ items, onChange }: { items: Education[]; onChange: (next: Education[]) => void }) {
  return (
    <>
      {items.map((entry, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label="Education"
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            Qualification or field of study
            <input value={entry.degree} onChange={(e) => onChange(patched(items, index, { degree: e.target.value }))} />
          </label>
          <label>
            School or institution
            <input value={entry.school} onChange={(e) => onChange(patched(items, index, { school: e.target.value }))} />
          </label>
          <label>
            Location
            <input value={entry.location} onChange={(e) => onChange(patched(items, index, { location: e.target.value }))} />
          </label>
          <DateField label="Start" value={entry.start} onChange={(start) => onChange(patched(items, index, { start }))} />
          <DateField
            label="End"
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
            Ongoing (Present)
          </label>
          <Bullets
            label="Achievement bullets"
            scope={`education ${index + 1}`}
            bullets={entry.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyEducation()])}>
        Add education
      </button>
    </>
  );
}

export function SkillsSection({ items, onChange }: { items: SkillGroup[]; onChange: (next: SkillGroup[]) => void }) {
  return (
    <>
      {items.map((skill, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label="Skill group"
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            Skill group
            <input value={skill.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <Bullets
            label="Achievement bullets"
            scope={`skill group ${index + 1}`}
            bullets={skill.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptySkill()])}>
        Add skill group
      </button>
    </>
  );
}

export function ProjectsSection({ items, onChange }: { items: ProjectEntry[]; onChange: (next: ProjectEntry[]) => void }) {
  return (
    <>
      {items.map((project, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label="Project"
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            Project name
            <input value={project.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <label>
            Context (optional)
            <input value={project.detail} onChange={(e) => onChange(patched(items, index, { detail: e.target.value }))} />
          </label>
          <Bullets
            label="Achievement bullets"
            scope={`project ${index + 1}`}
            bullets={project.bullets}
            onChange={(bullets) => onChange(patched(items, index, { bullets }))}
          />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyProject()])}>
        Add project
      </button>
    </>
  );
}

export function CertificationsSection({ items, onChange }: { items: Certification[]; onChange: (next: Certification[]) => void }) {
  return (
    <>
      {items.map((certification, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label="Certification"
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            Certification
            <input value={certification.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <label>
            Issuer (optional)
            <input value={certification.issuer} onChange={(e) => onChange(patched(items, index, { issuer: e.target.value }))} />
          </label>
          <DateField label="Date" value={certification.date} onChange={(date) => onChange(patched(items, index, { date }))} />
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyCertification()])}>
        Add certification
      </button>
    </>
  );
}

export function LanguagesSection({ items, onChange }: { items: Language[]; onChange: (next: Language[]) => void }) {
  return (
    <>
      {items.map((language, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label="Language"
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            Language
            <input value={language.name} onChange={(e) => onChange(patched(items, index, { name: e.target.value }))} />
          </label>
          <label>
            Level (optional)
            <input value={language.level} onChange={(e) => onChange(patched(items, index, { level: e.target.value }))} />
          </label>
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, emptyLanguage()])}>
        Add language
      </button>
    </>
  );
}

export function LinksSection({ items, onChange }: { items: Link[]; onChange: (next: Link[]) => void }) {
  return (
    <>
      {items.map((link, index) => (
        <fieldset className="entry" key={index}>
          <EntryLegend
            label="Link"
            index={index}
            count={items.length}
            onMove={(from, to) => onChange(moved(items, from, to))}
            onRemove={(position) => onChange(without(items, position))}
          />
          <label>
            Link label (optional)
            <input value={link.label} onChange={(e) => onChange(patched(items, index, { label: e.target.value }))} />
          </label>
          <label>
            Web address
            <input
              value={link.url}
              placeholder="https://example.com/portfolio"
              onChange={(e) => onChange(patched(items, index, { url: e.target.value }))}
            />
          </label>
        </fieldset>
      ))}
      <button type="button" className="add" onClick={() => onChange([...items, { label: '', url: '' }])}>
        Add link
      </button>
    </>
  );
}
