// Fixed application template. Everything from cv.json is plain data:
// user strings are only ever inserted as content values, never parsed as markup.
// The input is one CV plus the selected structure; the structure only changes
// which sections lead and how much employment detail shows, never the data.
#let input = json("cv.json")
#let cv = input.cv
#let structure = input.structure

// Blank (or whitespace-only) content must not produce headings or rows.
#let filled(s) = s != none and s != "" and s.trim() != ""

#set page(paper: "a4", margin: (x: 2.2cm, y: 1.8cm))
#set text(font: "Liberation Serif", size: 10.5pt, fill: black)
#set par(leading: 0.62em, justify: false, linebreaks: "optimized")

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
          if i < chars.len() { out += h(0.01pt, weak: true) }
      // 0pt weak spacing is optimized away and never breaks; 0.01pt does.
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

// Only the heading is sticky; sticking the body too would push a whole section
// to the next page and leave a nearly empty page behind.
#let section(title, body) = {
  block(above: 1.2em, below: 0.4em, width: 100%, sticky: true, {
    text(size: 11pt, weight: "bold", tracking: 0.05em, upper(title))
    v(2pt)
    line(length: 100%, stroke: 0.5pt)
    v(4pt)
  })
  body
}

#let bullet(body) = block(
  above: 1pt,
  below: 3pt,
  width: 100%,
  inset: (left: 1.05em),
  text([•] + h(0.35em)) + wrappable(body),
)

// A date never gains a month it does not have: year-only prints the year alone.
#let months = ("Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec")

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
    start + " – Present"
  } else {
    let end = fmt_date(e.end)
    if end == "" { start } else { start + " – " + end }
  }
}

#let dates_text(dates) = if dates == "" { } else {
  text(size: 9.5pt, fill: luma(40), wrappable("  (" + dates + ")"))
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
  parts.join([  ·  ])
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
  section("Skills", {
    for s in skill_groups {
      if filled(s.name) {
        block(above: 0.5em, below: 1pt, width: 100%, strong(wrappable(s.name)))
      }
      for b in s.bullets {
        if filled(b) { bullet(b) }
      }
    }
  })
}

#let roles_section() = {
  section("Work Experience", {
    for e in roles_shown {
      block(above: 0.5em, below: 1pt, width: 100%, {
        let head = ()
        if filled(e.title) { head.push(strong(wrappable(e.title))) }
        if filled(e.company) { head.push(wrappable(e.company)) }
        // Functional employment is a compact employer/title/date list; the
        // location and achievement bullets stay in the project, unrendered.
        if not compact_roles and filled(e.location) { head.push(wrappable(e.location)) }
        head.join([, ])
        dates_text(date_range(e))
      })
      if not compact_roles {
        for b in e.bullets {
          if filled(b) { bullet(b) }
        }
      }
    }
  })
}

// Header
#text(size: 19pt, weight: "bold", wrappable(cv.name))

#if contacts != [] {
  v(3pt)
  text(size: 9.5pt, fill: luma(40), contacts)
}

#if filled(cv.summary) {
  section("Professional Summary", wrappable(cv.summary))
}

// Leading skill evidence comes before employment in functional and combination.
#if skills_lead and skill_groups.len() > 0 { skills_section() }

#if roles_shown.len() > 0 { roles_section() }

#if schools.len() > 0 {
  section("Education", {
    for e in schools {
      block(above: 0.5em, below: 1pt, width: 100%, {
        let head = ()
        if filled(e.degree) { head.push(strong(wrappable(e.degree))) }
        if filled(e.school) { head.push(wrappable(e.school)) }
        if filled(e.location) { head.push(wrappable(e.location)) }
        head.join([, ])
        dates_text(date_range(e))
      })
      for b in e.bullets {
        if filled(b) { bullet(b) }
      }
    }
  })
}

// Reverse chronological keeps skills after the dated history as support.
#if not skills_lead and skill_groups.len() > 0 { skills_section() }

#if project_entries.len() > 0 {
  section("Projects", {
    for p in project_entries {
      block(above: 0.5em, below: 1pt, width: 100%, {
        if filled(p.name) { strong(wrappable(p.name)) }
        if filled(p.detail) {
          if filled(p.name) { [ — ] }
          wrappable(p.detail)
        }
      })
      for b in p.bullets {
        if filled(b) { bullet(b) }
      }
    }
  })
}

#if certs.len() > 0 {
  section("Certifications", {
    for c in certs {
      block(above: 1pt, below: 3pt, width: 100%, {
        let head = ()
        if filled(c.name) { head.push(strong(wrappable(c.name))) }
        if filled(c.issuer) { head.push(wrappable(c.issuer)) }
        head.join([, ])
        dates_text(fmt_date(c.date))
      })
    }
  })
}

#if langs.len() > 0 {
  section("Languages", {
    for l in langs {
      let label = if filled(l.name) and filled(l.level) {
        l.name + " — " + l.level
      } else {
        l.name + l.level
      }
      block(above: 1pt, below: 1pt, width: 100%, wrappable(label))
    }
  })
}
