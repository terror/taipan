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
  savedRevision: number;
}

export interface NotebookSessionOptions {
  generateIdentity?: () => string;
}

export interface SessionCell {
  identity: CellIdentity;
  cell: NotebookCell;
}

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
    savedRevision: 0,
  };
}

export function sessionCells(session: NotebookSession): SessionCell[] {
  return session.notebook.cells.map((cell, index) => ({
    identity: session.cellIdentities[index],
    cell,
  }));
}

export function replaceCellSource(
  session: NotebookSession,
  identity: CellIdentity,
  source: Source
): NotebookSession {
  const index = session.cellIdentities.indexOf(identity);

  if (
    index === -1 ||
    sourceText(session.notebook.cells[index].source) === sourceText(source)
  ) {
    return session;
  }

  const cell = session.notebook.cells[index];

  return replaceCell(session, index, { ...cell, source });
}

export function commitCellExecution(
  session: NotebookSession,
  identity: CellIdentity,
  outputs: NotebookOutput[],
  executionCount: ExecutionCount
): NotebookSession {
  const index = session.cellIdentities.indexOf(identity);

  if (index === -1) {
    return session;
  }

  const cell = session.notebook.cells[index];

  if (
    !isCodeCell(cell) ||
    (equal(cell.outputs, outputs) && cell.execution_count === executionCount)
  ) {
    return session;
  }

  return replaceCell(session, index, {
    ...cell,
    outputs,
    execution_count: executionCount,
  });
}

export function markNotebookSaved(
  session: NotebookSession,
  revision: number
): NotebookSession {
  if (
    revision === session.savedRevision ||
    revision < 0 ||
    revision > session.revision
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

function replaceCell(
  session: NotebookSession,
  index: number,
  cell: NotebookCell
): NotebookSession {
  return {
    ...session,
    notebook: {
      ...session.notebook,
      cells: session.notebook.cells.map((candidate, candidateIndex) =>
        candidateIndex === index ? cell : candidate
      ),
    },
    revision: session.revision + 1,
  };
}

function equal(left: unknown, right: unknown): boolean {
  return left === right || JSON.stringify(left) === JSON.stringify(right);
}
