export type Link = { label: string; url: string };

export type Experience = {
  title: string;
  company: string;
  location: string;
  start: string;
  // null means an ongoing role (Present); "" means no end was recorded.
  end: string | null;
  bullets: string[];
};

export type Cv = {
  name: string;
  email: string;
  phone: string;
  location: string;
  links: Link[];
  summary: string;
  experience: Experience[];
};

export type Structure = 'reverse-chronological' | 'functional' | 'combination';
export type ContentLanguage = 'english' | 'indonesian';

/** Mirrors the Rust Project struct; see docs/project-schema.md (version 1). */
export type Project = {
  version: 1;
  id: string;
  title: string;
  structure: Structure;
  content_language: ContentLanguage;
  updated_at: number;
  cv: Cv;
};

export type ProjectSummary = {
  id: string;
  title: string;
  name: string;
  updated_at: number;
};

export type ExportAck = { path: string; bytes: number; sha256: string };

export const emptyCv = (): Cv => ({
  name: '',
  email: '',
  phone: '',
  location: '',
  links: [],
  summary: '',
  experience: [],
});

export const emptyExperience = (): Experience => ({
  title: '',
  company: '',
  location: '',
  start: '',
  end: '',
  bullets: [],
});

/** A CV with no project title or full name yet is still identifiable in the library. */
export const projectLabel = (project: { title: string; name: string }): string =>
  project.title.trim() || project.name.trim() || 'Untitled CV';
