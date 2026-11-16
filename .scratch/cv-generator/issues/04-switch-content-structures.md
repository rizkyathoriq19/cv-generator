# 04 — Switch between all three structures without losing content

**What to build:** A job seeker authors skill-group achievements and switches one saved CV between reverse chronological, functional, and combination presentation without losing detailed employment or skill evidence.

**Blocked by:** 03 — Edit all career sections with precise dates and validation (satisfied).

**Status:** implemented and application-verified on Arch/Omarchy. Evidence: [ticket 04](../../evidence/ticket04/README.md).

**Story coverage:** US-11, US-22, US-23, US-24, US-25.

- [x] Edit named skill groups and user-authored achievement bullets through structured controls, with saved/reopened order and content.
  - Evidence: authored `Test automation` and `Communication` groups with bullets through the form; the stored JSON and reopened UI preserved the group order and both bullets. `skill_group_order_and_bullets_survive_save_and_reopen` pins the same round-trip.
- [x] Functional output leads with skill-group achievements and uses compact employer/title/date employment entries; combination leads with those achievements and retains detailed employment.
  - Evidence: `pdftotext` on UI-exported `functional-short.pdf` and `combination-short.pdf`; `functional_leads_with_skills_and_compacts_employment` and `combination_leads_with_skills_and_keeps_detailed_employment`.
- [x] Reverse chronological output continues to show detailed employment with skills as supporting content. All three structures share the accepted conservative presentation.
  - Evidence: `pdftotext` on `reverse-short.pdf`; `reverse_chronological_keeps_detailed_employment_before_supporting_skills`. All artifacts use the same A4 template, fonts, and section styling.
- [x] Switching structures never deletes, rewrites, or requires re-entry of hidden content. Reopen the project and switch back to demonstrate preservation.
  - Evidence: `switching_structure_preserves_content_through_save_and_reopen`; UI restart/reopen then Functional → Combination → Reverse (`28`–`32` screenshots) with the employment bullet, location, and both skill bullets still present.
- [x] Missing skill-group achievements produces a clear warning for functional/combination output, not an additional hard export requirement or generated career claims.
  - Evidence: `missing_skill_achievements_warn_only_for_leading_structures` proves the warning for functional/combination, no warning for reverse, `export_problem()` still `None`, and no generated bullet. The warning reuses the export warning list; it was not clicked through in the UI because the authored CV had skill bullets.
- [x] Use one shared CV data model and structure projections rather than independent generators with incompatible project data.
  - Evidence: one `Cv` model and one fixed template receive `{structure, cv}`; `one_cv_renders_a_different_document_per_structure` renders the same data three ways without a second model.
- [x] Exercise all three structures through UI selection, PDF preview/export, extraction checks, and save/reopen with both short and multipage fixtures.
  - Evidence: UI selection and preview for all three (`14`, `20`, `24`), native-dialog export of all three short and all three multipage artifacts (`18`, `23`, `27`, `36`, `38`, `40`), `pdfinfo`/`pdftotext` checks in the evidence README, and save/reopen of the short project (`28`–`32`). The multipage fixture was seeded into the isolated library so pagination could be exercised without hand-typing thousands of characters; it was opened, previewed (`2 page(s)`), and exported through the real UI.
