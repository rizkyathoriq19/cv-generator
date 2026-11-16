# 04 — Switch between all three structures without losing content

**What to build:** A job seeker authors skill-group achievements and switches one saved CV between reverse chronological, functional, and combination presentation without losing detailed employment or skill evidence.

**Blocked by:** 03 — Edit all career sections with precise dates and validation.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-11, US-22, US-23, US-24, US-25.

- [ ] Edit named skill groups and user-authored achievement bullets through structured controls, with saved/reopened order and content.
- [ ] Functional output leads with skill-group achievements and uses compact employer/title/date employment entries; combination leads with those achievements and retains detailed employment.
- [ ] Reverse chronological output continues to show detailed employment with skills as supporting content. All three structures share the accepted conservative presentation.
- [ ] Switching structures never deletes, rewrites, or requires re-entry of hidden content. Reopen the project and switch back to demonstrate preservation.
- [ ] Missing skill-group achievements produces a clear warning for functional/combination output, not an additional hard export requirement or generated career claims.
- [ ] Use one shared CV data model and structure projections rather than independent generators with incompatible project data.
- [ ] Exercise all three structures through UI selection, PDF preview/export, extraction checks, and save/reopen with both short and multipage fixtures.
