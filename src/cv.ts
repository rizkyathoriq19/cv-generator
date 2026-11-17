export type Link = { label: string; url: string };

/** A career date: month/year, year only, or no value. A missing month stays missing. */
export type CvDate = { year: number | null; month: number | null };

export type Experience = {
  title: string;
  company: string;
  location: string;
  start: CvDate;
  // null means an ongoing role (Present); an empty date means no end recorded.
  end: CvDate | null;
  bullets: string[];
};

export type Education = {
  school: string;
  degree: string;
  location: string;
  start: CvDate;
  end: CvDate | null;
  bullets: string[];
};

export type SkillGroup = { name: string; bullets: string[] };

export type ProjectEntry = { name: string; detail: string; bullets: string[] };

export type Certification = { name: string; issuer: string; date: CvDate };

export type Language = { name: string; level: string };

export type Cv = {
  name: string;
  /** One line under the name, e.g. "Full Stack Developer". Optional. */
  headline: string;
  email: string;
  phone: string;
  location: string;
  links: Link[];
  summary: string;
  experience: Experience[];
  education: Education[];
  skills: SkillGroup[];
  projects: ProjectEntry[];
  certifications: Certification[];
  languages: Language[];
};

export type Structure = 'reverse-chronological' | 'functional' | 'combination';

/** Presentation-only choices: switching never edits the CV content. */
export const STRUCTURES: Structure[] = ['reverse-chronological', 'functional', 'combination'];

export type Lang = 'english' | 'indonesian';

/** Mirrors the Rust Project struct; see docs/project-schema.md (version 1). */
export type Project = {
  version: 1;
  id: string;
  title: string;
  structure: Structure;
  content_language: Lang;
  updated_at: number;
  cv: Cv;
};

export type ProjectSummary = {
  id: string;
  title: string;
  name: string;
  updated_at: number;
  /** Null when no revision of the file is readable. */
  structure: Structure | null;
  content_language: Lang | null;
  /** The saved file is unreadable; opening it recovers or reports the damage. */
  damaged: boolean;
};

/** Present only when a damaged file was recovered from its previous revision. */
export type Recovery = {
  notice: string;
  preserved_as?: string;
};

export type Opened = {
  project: Project;
  recovery?: Recovery;
};

/** A save returns the stored project and, if the file was damaged mid-edit, a notice. */
export type SaveResult = {
  project: Project;
  notice?: string;
};

export type ExportAck = {
  path: string;
  bytes: number;
  sha256: string;
  warnings: string[];
};

export const emptyDate = (): CvDate => ({ year: null, month: null });

export const emptyCv = (): Cv => ({
  name: '',
  headline: '',
  email: '',
  phone: '',
  location: '',
  links: [],
  summary: '',
  experience: [],
  education: [],
  skills: [],
  projects: [],
  certifications: [],
  languages: [],
});

export const emptyExperience = (): Experience => ({
  title: '',
  company: '',
  location: '',
  start: emptyDate(),
  end: emptyDate(),
  bullets: [],
});

export const emptyEducation = (): Education => ({
  school: '',
  degree: '',
  location: '',
  start: emptyDate(),
  end: emptyDate(),
  bullets: [],
});

export const emptySkill = (): SkillGroup => ({ name: '', bullets: [] });
export const emptyProject = (): ProjectEntry => ({ name: '', detail: '', bullets: [] });
export const emptyCertification = (): Certification => ({ name: '', issuer: '', date: emptyDate() });
export const emptyLanguage = (): Language => ({ name: '', level: '' });

/** Move one item to another index: the only reordering mechanism in the editor. */
export const moved = <T,>(items: T[], from: number, to: number): T[] => {
  const next = items.slice();
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
};

/** A CV with no project title or full name yet is still identifiable in the library. */
export const projectLabel = (
  project: { title: string; name: string },
  fallback = 'Untitled CV',
): string => project.title.trim() || project.name.trim() || fallback;
