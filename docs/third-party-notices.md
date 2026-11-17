# Third-party notices

The application bundles only locally licensed assets; nothing is fetched at runtime.

| Component | Use | License | Notice |
| --- | --- | --- | --- |
| Typst 0.15.1 (`typst`, `typst-pdf`, `typst-layout`) | PDF generation and layout | Apache-2.0 | `LICENSE-Apache-2.0.txt` (installed beside this file) |
| PDF.js / pdfjs-dist 6.5.438 | Preview rendering (bundled `pdf.mjs`, `pdf.worker.min.mjs`) | Apache-2.0 | `LICENSE-Apache-2.0.txt` (installed beside this file) |
| Liberation Sans (Regular, Bold, Italic) | Bundled CV font | SIL Open Font License 1.1 (reserved name "Liberation") | `LICENSE-Liberation.txt` (installed beside this file) |
| Tauri 2 | Desktop shell | MIT / Apache-2.0 | `LICENSE-Apache-2.0.txt` (installed beside this file) |
| React / React DOM | UI shell, bundled into the frontend assets | MIT | `LICENSE-MIT-React.txt` (installed beside this file) |
| Vite | Bundling only; not redistributed | MIT | build-time tool |

Font redistribution: the Liberation fonts are distributed under the SIL OFL 1.1, which permits embedding and redistribution provided the license text accompanies the fonts and the reserved name is not used for modified versions. The fonts are bundled unmodified with their license file. The Apache-2.0 text ships for the Typst, PDF.js and Tauri components; the MIT text ships for the bundled React runtime; Vite is used only to build the frontend and is not redistributed.
