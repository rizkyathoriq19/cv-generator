// Fixed application template. Everything from cv.json is plain data:
// user strings are only ever inserted as content values, never parsed as markup.
#let cv = json("cv.json")

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
#let entry_blank(e) = (
  not filled(e.title) and not filled(e.company) and not filled(e.location)
    and not filled(e.start) and not filled(e.end)
    and e.bullets.all(b => not filled(b))
)

#let roles = cv.experience.filter(e => not entry_blank(e))

// Header
#text(size: 19pt, weight: "bold", wrappable(cv.name))

#if contacts != [] {
  v(3pt)
  text(size: 9.5pt, fill: luma(40), contacts)
}

#if filled(cv.summary) {
  section("Professional Summary", wrappable(cv.summary))
}

#if roles.len() > 0 {
  section("Work Experience", {
    for e in roles {
      block(above: 0.5em, below: 1pt, width: 100%, {
        let head = ()
        if filled(e.title) { head.push(strong(wrappable(e.title))) }
        if filled(e.company) { head.push(wrappable(e.company)) }
        if filled(e.location) { head.push(wrappable(e.location)) }

        // No end supplied is not already "Present": show the start alone.
        let dates = if not filled(e.start) {
          ""
        } else if e.end == none {
          e.start + " – Present"
        } else if filled(e.end) {
          e.start + " – " + e.end
        } else {
          e.start
        }

        head.join([, ])
        if dates != "" {
          text(size: 9.5pt, fill: luma(40), wrappable("  (" + dates + ")"))
        }
      })
      for b in e.bullets {
        if filled(b) { bullet(b) }
      }
    }
  })
}
