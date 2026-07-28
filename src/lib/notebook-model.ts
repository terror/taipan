export type JsonValue = null | boolean | number | string | JsonValue[] | JsonObject;

export interface JsonObject {
  [key: string]: JsonValue;
}

export type NotebookSource = string | string[];

export interface NotebookCell {
  cell_type: string;
  source: NotebookSource;
  metadata: JsonObject;
  id?: string;
  [key: string]: JsonValue | undefined;
}

export interface NotebookDocument {
  cells: NotebookCell[];
  metadata: JsonObject;
  nbformat: number;
  nbformat_minor: number;
  [key: string]: JsonValue | NotebookCell[];
}

export interface NotebookSession {
  path: string;
  notebook: NotebookDocument;
  revision: number;
  savedRevision: number;
}

export function createNotebookSession(path: string, notebook: NotebookDocument): NotebookSession {
  return { path, notebook, revision: 0, savedRevision: 0 };
}

export function sourceText(source: NotebookSource): string {
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

export function markSaved(session: NotebookSession, revision: number): NotebookSession {
  return { ...session, savedRevision: Math.max(session.savedRevision, revision) };
}

export function isDirty(session: NotebookSession): boolean {
  return session.revision !== session.savedRevision;
}

export function outputCount(cell: NotebookCell): number {
  return Array.isArray(cell.outputs) ? cell.outputs.length : 0;
}
