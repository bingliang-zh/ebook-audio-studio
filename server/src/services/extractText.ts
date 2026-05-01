import { extname } from "node:path";
import { readFile } from "node:fs/promises";

const textExtensions = new Set([".txt", ".md", ".markdown", ".html", ".htm"]);

export async function extractTextFromUpload(filePath: string, originalName: string): Promise<string> {
  const extension = extname(originalName).toLowerCase();

  if (textExtensions.has(extension)) {
    return readFile(filePath, "utf8");
  }

  if (extension === ".epub") {
    throw new Error("EPUB parsing is not wired yet. Add an EPUB parser in extractTextFromUpload.");
  }

  if (extension === ".pdf") {
    throw new Error("PDF parsing is not wired yet. Add a PDF parser in extractTextFromUpload.");
  }

  throw new Error("Unsupported file type. Upload .txt, .md, .html, .epub, or .pdf.");
}

