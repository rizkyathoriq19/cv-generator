import * as pdfjs from 'pdfjs-dist';
import workerSrc from 'pdfjs-dist/build/pdf.worker.min.mjs?url';

pdfjs.GlobalWorkerOptions.workerSrc = workerSrc;

/** Render the actual generated PDF bytes into one canvas per page. */
export async function showPdf(bytes: Uint8Array, host: HTMLElement): Promise<number> {
  const task = pdfjs.getDocument({ data: bytes });
  const doc = await task.promise;
  const dpr = window.devicePixelRatio || 1;
  const canvases: HTMLCanvasElement[] = [];

  try {
    for (let n = 1; n <= doc.numPages; n++) {
      const page = await doc.getPage(n);
      const viewport = page.getViewport({ scale: dpr });
      const canvas = document.createElement('canvas');
      canvas.className = 'page';
      canvas.width = Math.floor(viewport.width);
      canvas.height = Math.floor(viewport.height);
      canvas.style.width = `${viewport.width / dpr}px`;
      canvas.setAttribute('role', 'img');
      canvas.setAttribute('aria-label', `PDF page ${n} of ${doc.numPages}`);
      await page.render({ canvas, viewport }).promise;
      canvases.push(canvas);
    }
    host.replaceChildren(...canvases);
    return doc.numPages;
  } finally {
    // cleanup() only drops page caches; destroy() also releases the worker document.
    await task.destroy();
  }
}

export async function sha256(bytes: Uint8Array): Promise<string> {
  // Copy: pdf.js may transfer the buffer to its worker.
  const digest = await crypto.subtle.digest('SHA-256', bytes.slice());
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}
