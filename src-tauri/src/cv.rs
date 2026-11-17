
use serde::{Deserialize, Serialize};



// ---------------------------------------------------------------------------
// CV data (plain data only; never re-parsed as template code)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Link {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub url: String,
}

/// A career date with month/year precision, year-only precision, or no value.
/// `month` is only ever written when the user selected one: a missing month is
/// never materialized, so year-only records stay year-only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CvDate {
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub month: Option<u8>,
}

impl CvDate {
    pub(crate) fn is_empty(&self) -> bool {
        self.year.is_none()
    }

    /// Only provable contradictions: both years are known, or the same year with
    /// both months known. A year-only side never proves a month is wrong.
    pub(crate) fn contradicts(&self, end: &CvDate) -> bool {
        let (Some(start_year), Some(end_year)) = (self.year, end.year) else {
            return false;
        };
        if start_year != end_year {
            return start_year > end_year;
        }
        matches!((self.month, end.month), (Some(start), Some(end)) if start > end)
    }

    /// The documented contract is 1-12 and never a month without a year. A
    /// hand-edited file must fail with a message, not inside the template.
    pub(crate) fn shape_is_valid(&self) -> bool {
        self.month
            .is_none_or(|month| (1..=12).contains(&month) && self.year.is_some())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Experience {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub start: CvDate,
    /// `None` means an ongoing role (Present); an empty date means no end recorded.
    #[serde(default)]
    pub end: Option<CvDate>,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl Experience {
    /// A draft entry nobody has filled in yet: it must not reach the PDF.
    pub(crate) fn is_blank(&self) -> bool {
        [&self.title, &self.company, &self.location]
            .iter()
            .all(|text| text.trim().is_empty())
            && self.start.is_empty()
            && self.end.is_none_or(|end| end.is_empty())
            && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Education {
    #[serde(default)]
    pub school: String,
    #[serde(default)]
    pub degree: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub start: CvDate,
    #[serde(default)]
    pub end: Option<CvDate>,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl Education {
    pub(crate) fn is_blank(&self) -> bool {
        [&self.school, &self.degree, &self.location]
            .iter()
            .all(|text| text.trim().is_empty())
            && self.start.is_empty()
            && self.end.is_none_or(|end| end.is_empty())
            && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

/// A named skill group. Reverse chronological CVs use it as supporting content;
/// the group's achievements lead in functional and combination structures.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillGroup {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl SkillGroup {
    pub(crate) fn is_blank(&self) -> bool {
        self.name.trim().is_empty() && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

impl ProjectEntry {
    pub(crate) fn is_blank(&self) -> bool {
        [&self.name, &self.detail].iter().all(|text| text.trim().is_empty())
            && self.bullets.iter().all(|bullet| bullet.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Certification {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub date: CvDate,
}

impl Certification {
    pub(crate) fn is_blank(&self) -> bool {
        [&self.name, &self.issuer].iter().all(|text| text.trim().is_empty())
            && self.date.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LanguageEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub level: String,
}

impl LanguageEntry {
    pub(crate) fn is_blank(&self) -> bool {
        self.name.trim().is_empty() && self.level.trim().is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cv {
    #[serde(default)]
    pub name: String,
    /// One line under the name, e.g. "Full Stack Developer". Optional.
    #[serde(default)]
    pub headline: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub experience: Vec<Experience>,
    #[serde(default)]
    pub education: Vec<Education>,
    #[serde(default)]
    pub skills: Vec<SkillGroup>,
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
    #[serde(default)]
    pub certifications: Vec<Certification>,
    #[serde(default)]
    pub languages: Vec<LanguageEntry>,
}

impl Cv {
    /// Export needs an owner name and one substantive non-contact section.
    /// Messages follow the interface language, never the CV content language.
    pub(crate) fn export_problem(&self, lang: Language) -> Option<String> {
        if self.name.trim().is_empty() {
            return Some(lang.pick(
                "Add the CV owner's full name before exporting.",
                "Tambahkan nama lengkap pemilik CV sebelum mengekspor.",
            ));
        }
        let substantive = !self.summary.trim().is_empty()
            || self.experience.iter().any(|e| !e.is_blank())
            || self.education.iter().any(|e| !e.is_blank())
            || self.skills.iter().any(|s| !s.is_blank())
            || self.projects.iter().any(|p| !p.is_blank())
            || self.certifications.iter().any(|c| !c.is_blank())
            || self.languages.iter().any(|l| !l.is_blank());
        if !substantive {
            return Some(lang.pick(
                "Add a professional summary or at least one work experience, education, skill, \
                 project, certification, or language entry before exporting.",
                "Tambahkan ringkasan profesional atau setidaknya satu entri pengalaman kerja, \
                 pendidikan, keahlian, proyek, sertifikasi, atau bahasa sebelum mengekspor.",
            ));
        }
        if !self.email.trim().is_empty() && !valid_email(&self.email) {
            return Some(lang.pick(
                "The email address is not valid. Fix it or clear it before exporting.",
                "Alamat email tidak valid. Perbaiki atau kosongkan sebelum mengekspor.",
            ));
        }
        if let Some(link) = self.links.iter().find(|l| !l.url.trim().is_empty() && !valid_web_link(&l.url)) {
            let label = if link.label.trim().is_empty() {
                lang.pick("a link", "sebuah tautan")
            } else {
                format!("\"{}\"", link.label.trim())
            };
            return Some(lang.pick(
                format!(
                    "The web link for {label} is not valid: it must start with http:// or \
                     https:// and contain no spaces. Fix it or clear it before exporting."
                ),
                format!(
                    "Tautan web untuk {label} tidak valid: harus diawali http:// atau https:// \
                     dan tidak boleh mengandung spasi. Perbaiki atau kosongkan sebelum mengekspor."
                ),
            ));
        }
        if let Some(problem) = self.date_shape_problem(lang) {
            return Some(problem);
        }
        self.date_problem(lang)
    }

    /// At least one skill group carries a user-authored achievement bullet.
    /// A named group with no evidence is not a functional/combination lead.
    pub(crate) fn has_skill_achievements(&self) -> bool {
        self.skills
            .iter()
            .any(|group| group.bullets.iter().any(|bullet| !bullet.trim().is_empty()))
    }

    /// Warnings never block export: they name optional content the PDF will lack.
    /// They follow the interface language, independently of the CV content.
    pub(crate) fn export_warnings(&self, lang: Language, structure: Structure) -> Vec<String> {
        let mut warnings = Vec::new();
        if matches!(structure, Structure::Functional | Structure::Combination)
            && !self.has_skill_achievements()
        {
            let (name, id_name) = if structure == Structure::Functional {
                ("functional", "fungsional")
            } else {
                ("combination", "kombinasi")
            };
            warnings.push(lang.pick(
                format!(
                    "The {name} structure relies on skill-group achievements, but no skill group \
                     has an achievement bullet yet. Add achievements or choose another structure."
                ),
                format!(
                    "Struktur {id_name} mengandalkan pencapaian kelompok keahlian, tetapi belum ada \
                     kelompok keahlian yang memiliki butir pencapaian. Tambahkan pencapaian atau \
                     pilih struktur lain."
                ),
            ));
        }
        if self.email.trim().is_empty() && self.phone.trim().is_empty() {
            warnings.push(lang.pick(
                "No email or phone number is included, so the PDF gives employers no way to contact you.",
                "Tidak ada alamat email atau nomor telepon, sehingga pemberi kerja tidak memiliki \
                 cara untuk menghubungi Anda.",
            ));
        }
        let undated = self
            .experience
            .iter()
            .filter(|entry| !entry.is_blank())
            .map(|entry| entry.start.year.is_none())
            .chain(
                self.education
                    .iter()
                    .filter(|entry| !entry.is_blank())
                    .map(|entry| entry.start.year.is_none()),
            )
            .filter(|missing| *missing)
            .count();
        if undated > 0 {
            warnings.push(lang.pick(
                "Some entries have no start date, so they are listed after dated entries.",
                "Beberapa entri tidak memiliki tanggal mulai, sehingga ditampilkan setelah entri \
                 yang bertanggal.",
            ));
        }
        warnings
    }

    /// A month outside 1-12, or a month without a year, is corrupt data rather
    /// than user input; refuse it before it reaches the template.
    pub(crate) fn date_shape_problem(&self, lang: Language) -> Option<String> {
        let invalid = self
            .experience
            .iter()
            .map(|entry| ("work experience", "pengalaman kerja", &entry.start, entry.end.as_ref()))
            .chain(self.education.iter().map(|entry| {
                ("education", "pendidikan", &entry.start, entry.end.as_ref())
            }))
            .chain(
                self.certifications
                    .iter()
                    .map(|entry| ("certification", "sertifikasi", &entry.date, None)),
            )
            .find(|(_, _, start, end)| {
                !start.shape_is_valid() || end.is_some_and(|end| !end.shape_is_valid())
            });
        invalid.map(|(section, id_section, _, _)| {
            lang.pick(
                format!(
                    "Fix the dates for the {section}: a month must be between 1 and 12 and needs a year."
                ),
                format!(
                    "Perbaiki tanggal pada {id_section}: bulan harus antara 1 dan 12 dan memerlukan tahun."
                ),
            )
        })
    }

    pub(crate) fn date_problem(&self, lang: Language) -> Option<String> {
        let histories = self
            .experience
            .iter()
            .map(|entry| {
                (
                    "Work experience",
                    "Pengalaman kerja",
                    entry.title.as_str(),
                    &entry.start,
                    entry.end.as_ref(),
                )
            })
            .chain(self.education.iter().map(|entry| {
                (
                    "Education",
                    "Pendidikan",
                    entry.degree.as_str(),
                    &entry.start,
                    entry.end.as_ref(),
                )
            }));
        for (section, id_section, label, start, end) in histories {
            let Some(end) = end else { continue };
            if start.contradicts(end) {
                let name = if label.trim().is_empty() {
                    lang.pick("entry", "entri")
                } else {
                    format!("\"{}\"", label.trim())
                };
                return Some(lang.pick(
                    format!(
                        "Fix the dates for {section} {name}: the start date is after the end date."
                    ),
                    format!(
                        "Perbaiki tanggal untuk {id_section} {name}: tanggal mulai lebih akhir \
                         daripada tanggal selesai."
                    ),
                ));
            }
        }
        None
    }

    /// The PDF presents dated histories chronologically; project files keep the
    /// user's manual order so ties and imprecise dates are never re-ordered.
    pub(crate) fn render_view(&self) -> Cv {
        let mut view = self.clone();
        view.experience = ordered_histories(&self.experience, |e| &e.start, |e| e.end.is_none());
        view.education = ordered_histories(&self.education, |e| &e.start, |e| e.end.is_none());
        view
    }
}

pub(crate) fn valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.chars().any(char::is_whitespace) {
        return false;
    }
    let mut parts = email.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => {
            !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        _ => false,
    }
}

pub(crate) fn valid_web_link(url: &str) -> bool {
    let url = url.trim();
    if url.chars().any(char::is_whitespace) {
        return false;
    }
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
}

/// Ongoing entries with a known start first, then newest known start year, then
/// undated entries (ongoing or not) last. Within a year, months only establish
/// an order when every entry has one; otherwise the user's manual order is kept.
/// This avoids a comparator that pretends a year-only date is newer or older
/// than a month-precision one.
pub(crate) fn ordered_histories<T: Clone>(
    entries: &[T],
    start: impl Fn(&T) -> &CvDate,
    ongoing: impl Fn(&T) -> bool,
) -> Vec<T> {
    let mut out: Vec<T> = entries
        .iter()
        .filter(|entry| ongoing(entry) && start(entry).year.is_some())
        .cloned()
        .collect();

    let mut years: Vec<u16> = entries
        .iter()
        .filter(|entry| !ongoing(entry))
        .filter_map(|entry| start(entry).year)
        .collect();
    years.sort_unstable_by(|a, b| b.cmp(a));
    years.dedup();
    for year in years {
        let mut group: Vec<T> = entries
            .iter()
            .filter(|entry| !ongoing(entry) && start(entry).year == Some(year))
            .cloned()
            .collect();
        if group.iter().all(|entry| start(entry).month.is_some()) {
            group.sort_by_key(|entry| std::cmp::Reverse(start(entry).month));
        }
        out.extend(group);
    }

    out.extend(entries.iter().filter(|entry| start(entry).year.is_none()).cloned());
    out
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Structure {
    #[default]
    ReverseChronological,
    Functional,
    Combination,
}

/// Content and interface language. Both are user choices, independently
/// selectable: one decides PDF headings, the other decides the interface text.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    #[default]
    English,
    Indonesian,
}

impl Language {
    /// One message, two languages; `String` because messages carry values.
    pub(crate) fn pick(self, english: impl Into<String>, indonesian: impl Into<String>) -> String {
        match self {
            Language::English => english.into(),
            Language::Indonesian => indonesian.into(),
        }
    }
}
