import { invoke } from "@tauri-apps/api/core";
import type { Cell, Notebook } from "./types";

export interface NotebookSession {
  path: string;
  notebook: Notebook;
  revision: number;
  savedRevision: number;
}

export function openNotebook(path: string): Promise<Notebook> {
  return invoke<Notebook>("open_notebook", { path });
}

export function saveNotebook(path: string, notebook: Notebook): Promise<void> {
  return invoke("save_notebook", { path, notebook });
}

export function sourceText(source: Cell["source"]): string {
  return Array.isArray(source) ? source.join("") : source;
}

export function updateCellSource(
  session: NotebookSession,
  cellIndex: number,
  source: string,
): NotebookSession {
  const cell = session.notebook.cells[cellIndex];

  if (!cell || sourceText(cell.source) === source) {
    return session;
  }

  const cells = session.notebook.cells.map((candidate, index) =>
    index === cellIndex ? { ...candidate, source } : candidate,
  );

  return {
    ...session,
    notebook: { ...session.notebook, cells },
    revision: session.revision + 1,
  };
}
