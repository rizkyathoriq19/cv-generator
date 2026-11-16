# 05 — Duplicate CVs and separate interface/content languages

**What to build:** A job seeker duplicates a CV for another application or manual translation, selects Indonesian or English content headings, and changes interface language without mutating any CV.

**Blocked by:** 04 — Switch between all three structures without losing content.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-26, US-27, US-28, US-29.

- [ ] Duplicate any project as an independently identified CV including all content, structure, and selected content language; edits to either copy never affect the other.
- [ ] Allow Indonesian or English content language per CV, with corresponding standard PDF section headings and appropriate Present wording; authored content is never automatically translated.
- [ ] Provide Indonesian and English interface choices across the currently implemented library, editor, warnings, and preview/export actions. Later slices must extend the same language coverage to their controls.
- [ ] Interface language and content language are independent; switching interface language leaves CV content, content language, and exported career text unchanged.
- [ ] Demonstrate an Indonesian interface editing an English CV, and a duplicated manually translated Indonesian CV without synchronization.
- [ ] Save/reopen both copies and exercise all three structures in both content languages; verify no shared mutable project data or cross-project edits.
