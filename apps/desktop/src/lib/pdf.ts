/**
 * Turning a PDF into slide images, inside the app with PDF.js, so nothing
 * extra has to be installed on any computer.
 *
 * Each page is drawn 1920 pixels wide (the slide canvas width) and saved as
 * a JPEG, which keeps 60-page sermons a manageable size.
 */

import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

/** Width pages are drawn at, in pixels: the slide canvas width. */
const PAGE_WIDTH = 1920;

/**
 * Draws every page of the PDF at `url`, calling `onPage` with each page's
 * JPEG bytes in order. Resolves when all pages are done.
 */
export async function renderPdfPages(
  url: string,
  onPage: (index: number, total: number, jpeg: Uint8Array) => Promise<void>,
): Promise<number> {
  const loading = pdfjs.getDocument({ url });
  const document = await loading.promise;
  try {
    for (let number = 1; number <= document.numPages; number++) {
      const page = await document.getPage(number);
      const base = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: PAGE_WIDTH / base.width });

      const canvas = window.document.createElement("canvas");
      canvas.width = Math.round(viewport.width);
      canvas.height = Math.round(viewport.height);
      await page.render({ canvas, viewport }).promise;

      const blob = await new Promise<Blob | null>((resolve) =>
        canvas.toBlob(resolve, "image/jpeg", 0.9),
      );
      if (!blob) throw new Error(`Page ${number} could not be drawn.`);
      await onPage(number - 1, document.numPages, new Uint8Array(await blob.arrayBuffer()));
      page.cleanup();
    }
    return document.numPages;
  } finally {
    // Frees the document and its worker.
    await loading.destroy();
  }
}
