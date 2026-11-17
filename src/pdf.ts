import * as pdfjs from 'pdfjs-dist';
import workerSrc from 'pdfjs-dist/build/pdf.worker.min.mjs?url';

pdfjs.GlobalWorkerOptions.workerSrc = workerSrc;

/** Draw the actual generated PDF bytes into one canvas per page, off-document. */
export async function pdfPages(bytes: Uint8Array): Promise<{ pages: number; canvases: HTMLCanvasElement[] }> {
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
    // Returned detached: the caller inserts them only if this render is still
    // current, so a late result can never replace newer pages on screen.
    return { pages: doc.numPages, canvases };
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
