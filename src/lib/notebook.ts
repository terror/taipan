import { invoke } from '@tauri-apps/api/core';

import type {
  Attachments,
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

interface DocumentSnapshot {
  notebook: Notebook;
  cellIdentities: CellIdentity[];
  revision: number;
}

export interface NotebookSession {
  path: string;
  documentId: DocumentIdentity;
  notebook: Notebook;
  cellIdentities: CellIdentity[];
  revision: number;
  nextRevision: number;
  savedRevision: number;
  undoStack: DocumentSnapshot[];
  redoStack: DocumentSnapshot[];
  historyLimit: number;
  generateIdentity: () => string;
  generateCellId: () => string;
}

export interface NotebookSessionOptions {
  generateIdentity?: () => string;
  generateCellId?: () => string;
  historyLimit?: number;
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
      type: 'insert-cell';
      cell: SessionCell;
      after: CellIdentity | null;
    }
  | {
      type: 'delete-cell';
      cell: CellIdentity;
    }
  | {
      type: 'move-cell';
      cell: CellIdentity;
      after: CellIdentity | null;
    }
  | {
      type: 'set-cell-type';
      cell: CellIdentity;
      cellType: 'code' | 'markdown' | 'raw';
    }
  | {
      type: 'clear-outputs';
      cell: CellIdentity;
    }
  | {
      type: 'append-output';
      cell: CellIdentity;
      output: NotebookOutput;
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

const CELL_ID_PATTERN = /^[a-zA-Z0-9_-]{1,64}$/;
const DEFAULT_HISTORY_LIMIT = 100;
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
    generateUnique(generateIdentity, identities, 'session identity')
  );

  return {
    path,
    documentId: generateUnique(
      generateIdentity,
      identities,
      'document identity'
    ),
    notebook,
    cellIdentities,
    revision: 0,
    nextRevision: 1,
    savedRevision: 0,
    undoStack: [],
    redoStack: [],
    historyLimit: Math.max(
      0,
      Math.floor(options.historyLimit ?? DEFAULT_HISTORY_LIMIT)
    ),
    generateIdentity,
    generateCellId: options.generateCellId ?? defaultCellId,
  };
}

export function sessionCells(session: NotebookSession): SessionCell[] {
  return session.notebook.cells.map((cell, index) => ({
    identity: session.cellIdentities[index],
    cell,
  }));
}

export function createSessionCell(
  session: NotebookSession,
  cell: NotebookCell
): SessionCell {
  const identities = new Set(session.cellIdentities);
  const identity = generateUnique(
    session.generateIdentity,
    identities,
    'session identity'
  );

  return {
    identity,
    cell: ensurePersistedCellId(session, cell),
  };
}

export function applyTransaction(
  session: NotebookSession,
  operations: readonly NotebookOperation[]
): NotebookSession {
  const initial = snapshot(session);
  const result = operations.reduce(
    (current, operation) => applyOperation(session, current, operation),
    initial
  );

  if (
    initial === result ||
    (operations.length > 1 && snapshotsEqual(initial, result))
  ) {
    return session;
  }

  return {
    ...session,
    notebook: result.notebook,
    cellIdentities: result.cellIdentities,
    revision: session.nextRevision,
    nextRevision: session.nextRevision + 1,
    undoStack: pushBounded(session.undoStack, initial, session.historyLimit),
    redoStack: [],
  };
}

export function undo(session: NotebookSession): NotebookSession {
  const previous = session.undoStack.at(-1);

  if (!previous) {
    return session;
  }

  return {
    ...session,
    notebook: previous.notebook,
    cellIdentities: previous.cellIdentities,
    revision: previous.revision,
    undoStack: session.undoStack.slice(0, -1),
    redoStack: pushBounded(
      session.redoStack,
      snapshot(session),
      session.historyLimit
    ),
  };
}

export function redo(session: NotebookSession): NotebookSession {
  const next = session.redoStack.at(-1);

  if (!next) {
    return session;
  }

  return {
    ...session,
    notebook: next.notebook,
    cellIdentities: next.cellIdentities,
    revision: next.revision,
    undoStack: pushBounded(
      session.undoStack,
      snapshot(session),
      session.historyLimit
    ),
    redoStack: session.redoStack.slice(0, -1),
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

export function isValidCellId(id: string): boolean {
  return CELL_ID_PATTERN.test(id);
}

export function sourceText(source: Source | undefined): string {
  return Array.isArray(source) ? source.join('') : (source ?? '');
}

function defaultIdentity(): string {
  return crypto.randomUUID();
}

function defaultCellId(): string {
  return crypto.randomUUID().replaceAll('-', '').slice(0, 8);
}

function generateUnique(
  generate: () => string,
  existing: Set<string>,
  description: string
): string {
  for (let attempt = 0; attempt < GENERATION_LIMIT; attempt += 1) {
    const identity = generate();

    if (!existing.has(identity)) {
      existing.add(identity);
      return identity;
    }
  }

  throw new Error(`Unable to generate a unique ${description}`);
}

function ensurePersistedCellId(
  session: NotebookSession,
  cell: NotebookCell
): NotebookCell {
  const existing = new Set(
    session.notebook.cells
      .map((candidate) => candidate.id)
      .filter((id): id is string => id !== undefined && isValidCellId(id))
  );

  if (cell.id && isValidCellId(cell.id) && !existing.has(cell.id)) {
    return cell;
  }

  for (let attempt = 0; attempt < GENERATION_LIMIT; attempt += 1) {
    const id = session.generateCellId();

    if (!isValidCellId(id)) {
      throw new Error(`Generated invalid notebook cell ID: ${id}`);
    }

    if (!existing.has(id)) {
      return { ...cell, id };
    }
  }

  throw new Error('Unable to generate a unique notebook cell ID');
}

function snapshot(session: NotebookSession): DocumentSnapshot {
  return {
    notebook: session.notebook,
    cellIdentities: session.cellIdentities,
    revision: session.revision,
  };
}

function applyOperation(
  session: NotebookSession,
  snapshot: DocumentSnapshot,
  operation: NotebookOperation
): DocumentSnapshot {
  if (operation.type === 'insert-cell') {
    return insertCell(session, snapshot, operation.cell, operation.after);
  }

  const index = snapshot.cellIdentities.indexOf(operation.cell);

  if (index === -1) {
    return snapshot;
  }

  switch (operation.type) {
    case 'replace-source': {
      const cell = snapshot.notebook.cells[index];

      if (sourceText(cell.source) === sourceText(operation.source)) {
        return snapshot;
      }

      return replaceCell(snapshot, index, {
        ...cell,
        source: operation.source,
      });
    }
    case 'delete-cell':
      return {
        ...snapshot,
        notebook: {
          ...snapshot.notebook,
          cells: snapshot.notebook.cells.filter(
            (_, candidate) => candidate !== index
          ),
        },
        cellIdentities: snapshot.cellIdentities.filter(
          (_, candidate) => candidate !== index
        ),
      };
    case 'move-cell':
      return moveCell(snapshot, index, operation.after);
    case 'set-cell-type':
      return replaceCellType(snapshot, index, operation.cellType);
    case 'clear-outputs': {
      const cell = snapshot.notebook.cells[index];

      return isCodeCell(cell) && cell.outputs.length > 0
        ? replaceCell(snapshot, index, { ...cell, outputs: [] })
        : snapshot;
    }
    case 'append-output': {
      const cell = snapshot.notebook.cells[index];

      return isCodeCell(cell)
        ? replaceCell(snapshot, index, {
            ...cell,
            outputs: [...cell.outputs, operation.output],
          })
        : snapshot;
    }
    case 'replace-outputs': {
      const cell = snapshot.notebook.cells[index];

      return isCodeCell(cell) && !equal(cell.outputs, operation.outputs)
        ? replaceCell(snapshot, index, {
            ...cell,
            outputs: operation.outputs,
          })
        : snapshot;
    }
    case 'set-execution-count': {
      const cell = snapshot.notebook.cells[index];

      return isCodeCell(cell) &&
        cell.execution_count !== operation.executionCount
        ? replaceCell(snapshot, index, {
            ...cell,
            execution_count: operation.executionCount,
          })
        : snapshot;
    }
  }
}

function insertCell(
  session: NotebookSession,
  snapshot: DocumentSnapshot,
  sessionCell: SessionCell,
  after: CellIdentity | null
): DocumentSnapshot {
  if (snapshot.cellIdentities.includes(sessionCell.identity)) {
    return snapshot;
  }

  const anchor = after === null ? -1 : snapshot.cellIdentities.indexOf(after);

  if (after !== null && anchor === -1) {
    return snapshot;
  }

  const index = anchor + 1;
  const cells = snapshot.notebook.cells.slice();
  const cellIdentities = snapshot.cellIdentities.slice();
  const cell = ensurePersistedCellId(
    { ...session, notebook: snapshot.notebook },
    sessionCell.cell
  );

  cells.splice(index, 0, cell);
  cellIdentities.splice(index, 0, sessionCell.identity);

  return {
    ...snapshot,
    notebook: { ...snapshot.notebook, cells },
    cellIdentities,
  };
}

function moveCell(
  snapshot: DocumentSnapshot,
  index: number,
  after: CellIdentity | null
): DocumentSnapshot {
  const identity = snapshot.cellIdentities[index];

  if (after === identity) {
    return snapshot;
  }

  if (after !== null && !snapshot.cellIdentities.includes(after)) {
    return snapshot;
  }

  const cells = snapshot.notebook.cells.slice();
  const cellIdentities = snapshot.cellIdentities.slice();
  const [cell] = cells.splice(index, 1);

  cellIdentities.splice(index, 1);

  const target = after === null ? 0 : cellIdentities.indexOf(after) + 1;

  cells.splice(target, 0, cell);
  cellIdentities.splice(target, 0, identity);

  if (
    cellIdentities.every(
      (candidate, offset) => candidate === snapshot.cellIdentities[offset]
    )
  ) {
    return snapshot;
  }

  return {
    ...snapshot,
    notebook: { ...snapshot.notebook, cells },
    cellIdentities,
  };
}

function replaceCellType(
  snapshot: DocumentSnapshot,
  index: number,
  cellType: 'code' | 'markdown' | 'raw'
): DocumentSnapshot {
  const cell = snapshot.notebook.cells[index];

  if (cell.cell_type === cellType) {
    return snapshot;
  }

  const {
    attachments,
    execution_count: _executionCount,
    outputs: _outputs,
    ...common
  } = cell as NotebookCell & {
    attachments?: Attachments;
    execution_count?: ExecutionCount;
    outputs?: NotebookOutput[];
  };

  const replacement =
    cellType === 'code'
      ? {
          ...common,
          cell_type: cellType,
          execution_count: null,
          outputs: [],
          source: cell.source ?? '',
        }
      : {
          ...common,
          ...(attachments === undefined ? {} : { attachments }),
          cell_type: cellType,
          source: cell.source ?? '',
        };

  return replaceCell(snapshot, index, replacement as NotebookCell);
}

function replaceCell(
  snapshot: DocumentSnapshot,
  index: number,
  cell: NotebookCell
): DocumentSnapshot {
  return {
    ...snapshot,
    notebook: {
      ...snapshot.notebook,
      cells: snapshot.notebook.cells.map((candidate, candidateIndex) =>
        candidateIndex === index ? cell : candidate
      ),
    },
  };
}

function pushBounded<T>(items: T[], item: T, limit: number): T[] {
  return limit === 0 ? [] : [...items, item].slice(-limit);
}

function snapshotsEqual(
  left: DocumentSnapshot,
  right: DocumentSnapshot
): boolean {
  return (
    arrayEqual(left.cellIdentities, right.cellIdentities) &&
    notebookEqual(left.notebook, right.notebook)
  );
}

function arrayEqual<T>(left: readonly T[], right: readonly T[]): boolean {
  return (
    left === right ||
    (left.length === right.length &&
      left.every((value, index) => value === right[index]))
  );
}

function notebookEqual(left: Notebook, right: Notebook): boolean {
  if (left === right) {
    return true;
  }

  const leftRecord = left as unknown as Record<string, unknown>;
  const rightRecord = right as unknown as Record<string, unknown>;
  const keys = Object.keys(leftRecord);

  return (
    keys.length === Object.keys(rightRecord).length &&
    keys.every((key) =>
      key === 'cells'
        ? arrayEqualBy(left.cells, right.cells, cellEqual)
        : leftRecord[key] === rightRecord[key]
    )
  );
}

function arrayEqualBy<T>(
  left: readonly T[],
  right: readonly T[],
  compare: (left: T, right: T) => boolean
): boolean {
  return (
    left === right ||
    (left.length === right.length &&
      left.every((value, index) => compare(value, right[index])))
  );
}

function cellEqual(left: NotebookCell, right: NotebookCell): boolean {
  if (left === right) {
    return true;
  }

  const leftRecord = left as unknown as Record<string, unknown>;
  const rightRecord = right as unknown as Record<string, unknown>;
  const keys = Object.keys(leftRecord);

  return (
    keys.length === Object.keys(rightRecord).length &&
    keys.every((key) =>
      key === 'source'
        ? sourceText(left.source) === sourceText(right.source)
        : leftRecord[key] === rightRecord[key]
    )
  );
}

function equal(left: unknown, right: unknown): boolean {
  return left === right || JSON.stringify(left) === JSON.stringify(right);
}
