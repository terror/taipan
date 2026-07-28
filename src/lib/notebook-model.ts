import type { Cell, Notebook } from "./types";

export interface NotebookSession {
  path: string;
  notebook: Notebook;
  revision: number;
  savedRevision: number;
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
