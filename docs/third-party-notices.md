# Third-party notices

The application bundles only locally licensed assets; nothing is fetched at runtime.

| Component | Use | License | Notice |
| --- | --- | --- | --- |
| Typst 0.15.1 (`typst`, `typst-pdf`, `typst-layout`) | PDF generation and layout | Apache-2.0 | <https://github.com/typst/typst/blob/main/LICENSE> |
| PDF.js / pdfjs-dist 6.5.438 | Preview rendering (bundled `pdf.mjs`, `pdf.worker.min.mjs`) | Apache-2.0 | `node_modules/pdfjs-dist/LICENSE` |
| Liberation Serif (Regular, Bold, Italic) | Bundled CV font | SIL Open Font License 1.1 (reserved name "Liberation") | `src-tauri/fonts/LICENSE-LiberationSerif.txt` |
| Tauri 2 | Desktop shell | MIT / Apache-2.0 | <https://github.com/tauri-apps/tauri/blob/dev/LICENSE_APACHE-2.0> |
| React, Vite | UI shell and bundling (build-time) | MIT | package metadata |

Font redistribution: the Liberation fonts are distributed under the SIL OFL 1.1, which permits embedding and redistribution provided the license text accompanies the fonts and the reserved name is not used for modified versions. The fonts are bundled unmodified with their license file.
