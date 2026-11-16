# 09 — Keep PDF export current and the full editor keyboard-accessible

**What to build:** A job seeker edits rapidly, previews and exports only the current CV, handles rendering errors honestly, and completes the editor/preview workflow on narrow windows or without a mouse.

**Blocked by:** 04 — Switch between all three structures without losing content.

**Status:** draft — awaiting approval of granularity, blocking edges, and the test seam.

**Proposed triage:** ready-for-agent after approval. Execution still requires explicit implementation authorization.

**Story coverage:** US-30, US-31, US-32, US-33, US-34, US-35, US-36, US-37, US-48, US-52, US-53.

- [ ] Refresh the generated PDF after a short editing pause and label prior content outdated while generating; associate results with the exact current content revision.
- [ ] Delayed or out-of-order older results never replace a newer preview. Editing during export must not cause an old revision to be silently saved as the current CV.
- [ ] Reuse the previewed PDF artifact for export when current. If stale, generate/show the current artifact first; rendering failure preserves an older preview visibly outdated and blocks exporting it as current.
- [ ] Confirm destination overwrites and preserve existing files on cancelled or failed PDF writes, with accessible error feedback rather than partial-success output.
- [ ] Exercise all three structures with long content and page-boundary headings; verify A4, intended reading order, selectable text, no clipping/loss/shrinking, and current preview/export byte identity.
- [ ] Wide windows show forms and preview together; narrow windows provide usable switchable tabs without a separate HTML document preview.
- [ ] Core library/editor/preview controls work by keyboard with labels, visible focus, accessible validation/render errors, and reorder buttons. No task requires drag-and-drop.
- [ ] Add deterministic late-render and failed-render/write checks through the smallest test-only boundary, plus an actual desktop keyboard/narrow-window walkthrough. Later feature and platform gates must include their newly added controls.
