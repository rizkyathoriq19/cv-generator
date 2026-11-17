// Fixed application template. Everything from cv.json is plain data:
// user strings are only ever inserted as content values, never parsed as markup.
// The input is one CV plus the selected structure; the structure only changes
// which sections lead and how much employment detail shows, never the data.
#let input = json("cv.json")
#let cv = input.cv
#let structure = input.structure
// Standard headings and the Present wording follow the CV content language.
// Authored text is copied verbatim: this template never translates content.
#let indonesian = input.language == "indonesian"
#let word(en, id) = if indonesian { id } else { en }

// Blank (or whitespace-only) content must not produce headings or rows.
#let filled(s) = s != none and s != "" and s.trim() != ""

// One conservative presentation for every structure: a single column, real
// headings (PDF bookmarks), selectable text, black ink plus one grey for
// secondary facts (employer, dates, contacts). No icons, tables, or columns,
// so text extraction follows reading order.
#let ink = rgb("#1a1a1a")
#let quiet = rgb("#6b6b6b")

#set document(title: if filled(cv.name) { cv.name } else { "CV" })
// Page numbers only when there is more than one page; contact details never
// go in the footer.
#set page(
  paper: "a4",
  margin: (x: 2cm, top: 1.8cm, bottom: 2cm),
  footer: context {
    let total = counter(page).final().first()
    if total > 1 {
      align(right, text(size: 8.5pt, fill: quiet, counter(page).display("1 / 1", both: true)))
    }
  },
)
#set text(font: "Liberation Sans", size: 10pt, fill: ink, lang: if indonesian { "id" } else { "en" })
#set par(leading: 0.6em, spacing: 0.6em, justify: false, linebreaks: "optimized")
#set list(marker: text(size: 8pt, baseline: -0.5pt)[•], indent: 0.2em, body-indent: 0.7em, spacing: 0.5em)
#show list: set block(above: 0.5em, below: 0.2em)

// Only the heading is sticky; sticking the body too would push a whole section
// to the next page and leave a nearly empty page behind.
// No letter-spacing: even 0.1em makes extractors split a heading into
// fragments ("PRO FESSIO NA L"), which ATS parsers then miss.
#show heading.where(level: 1): it => block(
  above: 1.7em,
  below: 0.75em,
  width: 100%,
  sticky: true,
  inset: (bottom: 4pt),
  stroke: (bottom: (paint: quiet, thickness: 0.6pt, dash: "dotted")),
  text(size: 10.5pt, weight: "bold", upper(it.body)),
)

// Typst does not break overlong tokens. Split them into chunks joined by
// zero-width weak spacing so long words and URLs wrap instead of overflowing.
// ponytail: fixed 24-cluster chunks; tune only if a font/size change demands it.
#let wrappable(s, chunk: 24) = {
  let out = []
  let lines = s.split("\n")
  for (li, line) in lines.enumerate() {
    let words = line.split(" ")
    for (wi, w) in words.enumerate() {
      let chars = w.clusters()
      if chars.len() > chunk {
        let i = 0
        while i < chars.len() {
          out += text(chars.slice(i, calc.min(i + chunk, chars.len())).join())
          i += chunk
          // 0pt weak spacing is optimized away and never breaks; 0.01pt does.
          if i < chars.len() { out += h(0.01pt, weak: true) }
        }
      } else {
        out += text(w)
      }
      if wi < words.len() - 1 { out += [ ] }
    }
    if li < lines.len() - 1 { out += linebreak() }
  }
  out
}

#let section(title, body) = {
  heading(level: 1, title)
  body
}

#let bullets(items) = {
  let shown = items.filter(filled)
  if shown.len() > 0 { list(..shown.map(wrappable)) }
}

// A date never gains a month it does not have: year-only prints the year alone.
// Month abbreviations are generated text, so they follow the content language.
#let months = if indonesian {
  ("Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des")
} else {
  ("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")
}

#let fmt_date(d) = {
  if d == none or d.year == none {
    ""
  } else if d.month != none {
    months.at(d.month - 1) + " " + str(d.year)
  } else {
    str(d.year)
  }
}

#let date_empty(d) = d == none or d.year == none

// No end supplied is not already "Present": show the start alone.
#let date_range(e) = {
  let start = fmt_date(e.start)
  if start == "" {
    ""
  } else if e.end == none {
    start + " – " + word("Present", "Sekarang")
  } else {
    let end = fmt_date(e.end)
    if end == "" { start } else { start + " – " + end }
  }
}

// An entry heading: the first filled part leads in bold, dates sit on the
// right of the same line, and the remaining parts follow on a second line.
// Plain text in reading order, no table, so extraction stays linear.
#let entry(parts, dates, above: 0.85em, second: true) = {
  let parts = parts.filter(filled)
  block(above: above, below: 0.3em, width: 100%, sticky: true, {
    if parts.len() > 0 { strong(wrappable(parts.first())) }
    if not second and parts.len() > 1 { [, ] + parts.slice(1).map(wrappable).join([, ]) }
    if dates != "" {
      // A space before the fill lets long headings push the dates to the next
      // line, still right-aligned, instead of colliding with them.
      [ ] + h(1fr) + text(fill: quiet, dates)
    }
    if second and parts.len() > 1 {
      linebreak()
      text(fill: quiet, parts.slice(1).map(wrappable).join([, ]))
    }
  })
}

#let contacts = {
  let parts = ()
  if filled(cv.email) { parts.push(wrappable(cv.email)) }
  if filled(cv.phone) { parts.push(wrappable(cv.phone)) }
  if filled(cv.location) { parts.push(wrappable(cv.location)) }
  for l in cv.links {
    if filled(l.url) {
      parts.push(if filled(l.label) { wrappable(l.label + ": " + l.url) } else { wrappable(l.url) })
    }
  }
  parts.join(text(fill: quiet)[#h(0.5em)·#h(0.5em)])
}

// A draft entry with no content at all never becomes a row.
#let role_blank(e) = (
  not filled(e.title) and not filled(e.company) and not filled(e.location)
    and date_empty(e.start) and date_empty(e.end)
    and e.bullets.all(b => not filled(b))
)

#let school_blank(e) = (
  not filled(e.school) and not filled(e.degree) and not filled(e.location)
    and date_empty(e.start) and date_empty(e.end)
    and e.bullets.all(b => not filled(b))
)

#let skill_blank(s) = not filled(s.name) and s.bullets.all(b => not filled(b))
#let project_blank(p) = not filled(p.name) and not filled(p.detail) and p.bullets.all(b => not filled(b))
#let cert_blank(c) = not filled(c.name) and not filled(c.issuer) and date_empty(c.date)
#let language_blank(l) = not filled(l.name) and not filled(l.level)

#let roles = cv.experience.filter(e => not role_blank(e))
#let schools = cv.education.filter(e => not school_blank(e))
#let skill_groups = cv.skills.filter(s => not skill_blank(s))
#let project_entries = cv.projects.filter(p => not project_blank(p))
#let certs = cv.certifications.filter(c => not cert_blank(c))
#let langs = cv.languages.filter(l => not language_blank(l))

// Functional and combination lead with authored skill evidence; reverse
// chronological leads with detailed employment and keeps skills as support.
#let skills_lead = structure == "functional" or structure == "combination"
#let compact_roles = structure == "functional"

// Functional hides location and bullets, so an entry carrying only those has
// no compact row; it must not render an empty block or heading.
#let roles_shown = if compact_roles {
  roles.filter(e => filled(e.title) or filled(e.company) or not date_empty(e.start))
} else {
  roles
}

#let skills_section() = {
  section(word("Skills", "Keahlian"), {
    for (i, s) in skill_groups.enumerate() {
      if filled(s.name) {
        block(above: if i == 0 { 0em } else { 0.75em }, below: 0.3em, sticky: true, strong(wrappable(s.name)))
      }
      bullets(s.bullets)
    }
  })
}

#let roles_section() = {
  section(word("Work Experience", "Pengalaman Kerja"), {
    for (i, e) in roles_shown.enumerate() {
      // Functional employment is a compact employer/title/date list; the
      // location and achievement bullets stay in the project, unrendered.
      if compact_roles {
        entry((e.title, e.company), date_range(e), above: if i == 0 { 0em } else { 0.4em }, second: false)
      } else {
        entry((e.title, e.company, e.location), date_range(e), above: if i == 0 { 0em } else { 1.05em })
        bullets(e.bullets)
      }
    }
  })
}

// Header: name, then one quiet contact line. Contact details stay in the body,
// never in page headers or footers.
#block(below: 0pt, text(size: 22pt, weight: "bold", wrappable(cv.name)))

#if filled(cv.headline) {
  block(above: 9pt, below: 0pt, text(size: 11.5pt, wrappable(cv.headline)))
}

#if contacts != none and contacts != [] {
  block(above: 9pt, below: 0.4em, text(size: 9.5pt, fill: quiet, contacts))
}

#if filled(cv.summary) {
  section(word("Professional Summary", "Ringkasan Profesional"), wrappable(cv.summary))
}

// Leading skill evidence comes before employment in functional and combination.
#if skills_lead and skill_groups.len() > 0 { skills_section() }

#if roles_shown.len() > 0 { roles_section() }

#if schools.len() > 0 {
  section(word("Education", "Pendidikan"), {
    for (i, e) in schools.enumerate() {
      entry((e.degree, e.school, e.location), date_range(e), above: if i == 0 { 0em } else { 1.05em })
      bullets(e.bullets)
    }
  })
}

// Reverse chronological keeps skills after the dated history as support.
#if not skills_lead and skill_groups.len() > 0 { skills_section() }

#if project_entries.len() > 0 {
  section(word("Projects", "Proyek"), {
    for (i, p) in project_entries.enumerate() {
      entry((p.name, p.detail), "", above: if i == 0 { 0em } else { 1.05em })
      bullets(p.bullets)
    }
  })
}

#if certs.len() > 0 {
  section(word("Certifications", "Sertifikasi"), {
    for (i, c) in certs.enumerate() {
      entry((c.name, c.issuer), fmt_date(c.date), above: if i == 0 { 0em } else { 0.7em })
    }
  })
}

#if langs.len() > 0 {
  section(word("Languages", "Bahasa"), {
    langs
      .map(l => if filled(l.name) and filled(l.level) {
        wrappable(l.name + " — " + l.level)
      } else {
        wrappable(l.name + l.level)
      })
      .join(text(fill: quiet)[#h(0.6em)·#h(0.6em)])
  })
}
