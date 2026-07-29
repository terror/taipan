import { invoke } from '@tauri-apps/api/core';

import type {
  CodeCell,
  ExecutionCount,
  MarkdownCell,
  Notebook,
  NotebookCell,
  NotebookOutput,
  Source,
} from './types';

export type DocumentIdentity = string;
export type CellIdentity = string;

export interface NotebookSession {
  path: string;
  documentId: DocumentIdentity;
  notebook: Notebook;
  cellIdentities: CellIdentity[];
  revision: number;
  nextRevision: number;
  savedRevision: number;
}

export interface NotebookSessionOptions {
  generateIdentity?: () => string;
}

export interface SessionCell {
  identity: CellIdentity;
  cell: NotebookCell;
}

export type NotebookOperation =
  | {
      type: 'replace-source';
      cell: CellIdentity;
      source: Source;
    }
  | {
      type: 'replace-outputs';
      cell: CellIdentity;
      outputs: NotebookOutput[];
    }
  | {
      type: 'set-execution-count';
      cell: CellIdentity;
      executionCount: ExecutionCount;
    };

const GENERATION_LIMIT = 1_000;

export function openNotebook(path: string): Promise<Notebook> {
  return invoke<Notebook>('open_notebook', { path });
}

export function saveNotebook(path: string, notebook: Notebook): Promise<void> {
  return invoke('save_notebook', { path, notebook });
}

export function createNotebookSession(
  path: string,
  notebook: Notebook,
  options: NotebookSessionOptions = {}
): NotebookSession {
  const generateIdentity = options.generateIdentity ?? defaultIdentity;
  const identities = new Set<CellIdentity>();
  const cellIdentities = notebook.cells.map(() =>
    generateUnique(generateIdentity, identities)
  );

  return {
    path,
    documentId: generateUnique(generateIdentity, identities),
    notebook,
    cellIdentities,
    revision: 0,
    nextRevision: 1,
    savedRevision: 0,
  };
}

export function sessionCells(session: NotebookSession): SessionCell[] {
  return session.notebook.cells.map((cell, index) => ({
    identity: session.cellIdentities[index],
    cell,
  }));
}

export function applyTransaction(
  session: NotebookSession,
  operations: readonly NotebookOperation[]
): NotebookSession {
  const notebook = operations.reduce(
    (notebook, operation) =>
      applyOperation(notebook, session.cellIdentities, operation),
    session.notebook
  );

  if (notebook === session.notebook) {
    return session;
  }

  return {
    ...session,
    notebook,
    revision: session.nextRevision,
    nextRevision: session.nextRevision + 1,
  };
}

export function markNotebookSaved(
  session: NotebookSession,
  revision: number
): NotebookSession {
  if (
    revision === session.savedRevision ||
    revision < 0 ||
    revision >= session.nextRevision
  ) {
    return session;
  }

  return { ...session, savedRevision: revision };
}

export function isCodeCell(cell: NotebookCell): cell is CodeCell {
  return cell.cell_type === 'code';
}

export function isMarkdownCell(cell: NotebookCell): cell is MarkdownCell {
  return cell.cell_type === 'markdown';
}

export function sourceText(source: Source | undefined): string {
  return Array.isArray(source) ? source.join('') : (source ?? '');
}

function defaultIdentity(): string {
  return crypto.randomUUID();
}

function generateUnique(generate: () => string, existing: Set<string>): string {
  for (let attempt = 0; attempt < GENERATION_LIMIT; attempt += 1) {
    const identity = generate();

    if (!existing.has(identity)) {
      existing.add(identity);
      return identity;
    }
  }

  throw new Error('Unable to generate a unique session identity');
}

function applyOperation(
  notebook: Notebook,
  cellIdentities: CellIdentity[],
  operation: NotebookOperation
): Notebook {
  const index = cellIdentities.indexOf(operation.cell);

  if (index === -1) {
    return notebook;
  }

  const cell = notebook.cells[index];

  switch (operation.type) {
    case 'replace-source':
      return sourceText(cell.source) === sourceText(operation.source)
        ? notebook
        : replaceCell(notebook, index, {
            ...cell,
            source: operation.source,
          });
    case 'replace-outputs':
      return isCodeCell(cell) && !equal(cell.outputs, operation.outputs)
        ? replaceCell(notebook, index, {
            ...cell,
            outputs: operation.outputs,
          })
        : notebook;
    case 'set-execution-count':
      return isCodeCell(cell) &&
        cell.execution_count !== operation.executionCount
        ? replaceCell(notebook, index, {
            ...cell,
            execution_count: operation.executionCount,
          })
        : notebook;
  }
}

function replaceCell(
  notebook: Notebook,
  index: number,
  cell: NotebookCell
): Notebook {
  return {
    ...notebook,
    cells: notebook.cells.map((candidate, candidateIndex) =>
      candidateIndex === index ? cell : candidate
    ),
  };
}

function equal(left: unknown, right: unknown): boolean {
  return left === right || JSON.stringify(left) === JSON.stringify(right);
}
