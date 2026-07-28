import { describe, expect, test } from 'bun:test';

import {
  type NotebookOperation,
  type NotebookSession,
  applyTransaction,
  createNotebookSession,
  createSessionCell,
  isCodeCell,
  isValidCellId,
  markNotebookSaved,
  redo,
  sessionCells,
  sourceText,
  undo,
} from '../src/lib/notebook';
import type {
  CodeCell,
  Notebook,
  NotebookCell,
  NotebookOutput,
  UnknownCell,
} from '../src/lib/types';

type Fixture = Omit<Notebook, 'cells'> & {
  cells: [CodeCell & Record<string, unknown>, NotebookCell];
  unknown: string;
};

function counter(prefix: string): () => string {
  let value = 0;
  return () => `${prefix}-${value++}`;
}

function output(text: string): NotebookOutput {
  return { name: 'stdout', output_type: 'stream', text };
}

function fixture(): Fixture {
  return {
    cells: [
      {
        cell_type: 'code',
        custom: { foo: true },
        execution_count: null,
        metadata: { foo: 'bar' },
        outputs: [output('foo')],
        source: ['foo\n', 'bar'],
      },
      {
        cell_type: 'markdown',
        id: 'markdown-id',
        metadata: {},
        source: 'baz',
      },
    ],
    metadata: { custom: { foo: true } },
    nbformat: 4,
    nbformat_minor: 5,
    unknown: 'preserved',
  };
}

function open(
  options: { historyLimit?: number; generateCellId?: () => string } = {}
): NotebookSession {
  return createNotebookSession('foo.ipynb', fixture(), {
    generateIdentity: counter('session'),
    generateCellId: options.generateCellId ?? counter('cell'),
    historyLimit: options.historyLimit,
  });
}

function identity(session: NotebookSession, index: number): string {
  return session.cellIdentities[index];
}

function transact(
  session: NotebookSession,
  ...operations: NotebookOperation[]
): NotebookSession {
  return applyTransaction(session, operations);
}

function code(session: NotebookSession, index = 0): CodeCell {
  const cell = session.notebook.cells[index];

  if (!isCodeCell(cell)) {
    throw new Error(`Expected cell ${index} to be code`);
  }

  return cell;
}

function expectCell(
  session: NotebookSession,
  index: number,
  expected: unknown
): void {
  expect(session.notebook.cells[index] as unknown).toEqual(expected);
}

describe('notebook document identities', () => {
  test('are stable and never enter serialized notebook content', () => {
    const notebook = fixture();
    const session = createNotebookSession('foo.ipynb', notebook, {
      generateIdentity: counter('identity'),
    });

    expect(session.documentId).toBe('identity-2');
    expect(session.cellIdentities).toEqual(['identity-0', 'identity-1']);
    expect(session.notebook).toBe(notebook);
    expect(session.notebook.cells[0].id).toBeUndefined();
    expect(session.notebook.cells[1].id).toBe('markdown-id');
    expect(JSON.stringify(session.notebook)).not.toContain('identity-');

    const edited = transact(session, {
      type: 'replace-source',
      cell: identity(session, 0),
      source: 'bar',
    });

    expect(edited.documentId).toBe(session.documentId);
    expect(edited.cellIdentities).toEqual(session.cellIdentities);
  });

  test('gives new cells valid deterministic persisted IDs', () => {
    const session = open();
    const cell = createSessionCell(session, {
      cell_type: 'markdown',
      metadata: {},
      source: 'foo',
    });
    const inserted = transact(session, {
      type: 'insert-cell',
      cell,
      after: identity(session, 0),
    });

    expect(cell.cell.id).toBe('cell-0');
    expect(isValidCellId(cell.cell.id ?? '')).toBe(true);
    expect(sessionCells(inserted)[1]).toEqual(cell);
  });

  test('retains valid IDs and replaces missing, invalid, or duplicate IDs', () => {
    function check(id: string | undefined, expected: string) {
      const cell = createSessionCell(open(), {
        cell_type: 'raw',
        id,
        metadata: {},
        source: '',
      });

      expect(cell.cell.id).toBe(expected);
      expect(isValidCellId(cell.cell.id ?? '')).toBe(true);
    }

    check('foo_ID-1', 'foo_ID-1');
    check('invalid.id', 'cell-0');
    check('markdown-id', 'cell-0');
    check(undefined, 'cell-0');

    expect(isValidCellId('a'.repeat(64))).toBe(true);
    expect(isValidCellId('a'.repeat(65))).toBe(false);
    expect(isValidCellId(':')).toBe(false);
  });

  test('rejects invalid generated IDs', () => {
    const session = open({ generateCellId: () => 'invalid.id' });

    expect(() =>
      createSessionCell(session, {
        cell_type: 'raw',
        metadata: {},
        source: '',
      })
    ).toThrow('Generated invalid notebook cell ID');
  });
});

describe('notebook document operations', () => {
  test('replaces source by identity after cells move', () => {
    const session = open();
    const first = identity(session, 0);
    const moved = transact(session, {
      type: 'move-cell',
      cell: first,
      after: identity(session, 1),
    });
    const edited = transact(moved, {
      type: 'replace-source',
      cell: first,
      source: 'bar',
    });

    expect(edited.notebook.cells[1]).toEqual({
      ...session.notebook.cells[0],
      source: 'bar',
    });
    expect(edited.notebook.metadata).toBe(session.notebook.metadata);
    expect('unknown' in edited.notebook && edited.notebook.unknown).toBe(
      'preserved'
    );
    expect(code(edited, 1).outputs).toBe(code(session).outputs);
  });

  test('inserts, moves, and deletes cells by identity', () => {
    const session = open();
    const first = identity(session, 0);
    const second = identity(session, 1);
    const cell = createSessionCell(session, {
      cell_type: 'raw',
      metadata: {},
      source: 'qux',
    });
    const inserted = transact(session, {
      type: 'insert-cell',
      cell,
      after: first,
    });
    const moved = transact(inserted, {
      type: 'move-cell',
      cell: cell.identity,
      after: null,
    });
    const deleted = transact(moved, { type: 'delete-cell', cell: first });

    expect(inserted.cellIdentities).toEqual([first, cell.identity, second]);
    expect(moved.cellIdentities).toEqual([cell.identity, first, second]);
    expect(deleted.cellIdentities).toEqual([cell.identity, second]);
    expect(
      deleted.notebook.cells.map(({ source }) => sourceText(source))
    ).toEqual(['qux', 'baz']);
  });

  test('changes cell type while retaining custom fields and identity', () => {
    const session = open();
    const first = identity(session, 0);
    const markdown = transact(session, {
      type: 'set-cell-type',
      cell: first,
      cellType: 'markdown',
    });

    expect(markdown.cellIdentities[0]).toBe(first);
    expectCell(markdown, 0, {
      cell_type: 'markdown',
      custom: { foo: true },
      metadata: { foo: 'bar' },
      source: ['foo\n', 'bar'],
    });

    const restored = transact(markdown, {
      type: 'set-cell-type',
      cell: first,
      cellType: 'code',
    });

    expectCell(restored, 0, {
      cell_type: 'code',
      custom: { foo: true },
      execution_count: null,
      metadata: { foo: 'bar' },
      outputs: [],
      source: ['foo\n', 'bar'],
    });
  });

  test('clears, appends, and replaces outputs', () => {
    const session = open();
    const first = identity(session, 0);
    const cleared = transact(session, { type: 'clear-outputs', cell: first });
    const appended = transact(cleared, {
      type: 'append-output',
      cell: first,
      output: output('bar'),
    });
    const replaced = transact(appended, {
      type: 'replace-outputs',
      cell: first,
      outputs: [output('baz'), output('qux')],
    });

    expect(code(cleared).outputs).toEqual([]);
    expect(code(appended).outputs).toEqual([output('bar')]);
    expect(code(replaced).outputs).toEqual([output('baz'), output('qux')]);
  });

  test('sets execution count', () => {
    const session = open();
    const edited = transact(session, {
      type: 'set-execution-count',
      cell: identity(session, 0),
      executionCount: 42,
    });

    expect(code(edited).execution_count).toBe(42);
  });

  test('ignores no-ops and stale identities', () => {
    const session = open();
    const first = identity(session, 0);
    const second = identity(session, 1);

    function check(...operations: NotebookOperation[]) {
      expect(transact(session, ...operations)).toBe(session);
    }

    check();
    check({ type: 'replace-source', cell: first, source: 'foo\nbar' });
    check({ type: 'move-cell', cell: first, after: null });
    check({ type: 'set-cell-type', cell: first, cellType: 'code' });
    check({ type: 'clear-outputs', cell: second });
    check({ type: 'replace-outputs', cell: first, outputs: [output('foo')] });
    check({ type: 'set-execution-count', cell: first, executionCount: null });
    check({ type: 'delete-cell', cell: 'stale' });
    check({ type: 'replace-source', cell: 'stale', source: 'bar' });
    check(
      { type: 'replace-source', cell: first, source: 'bar' },
      { type: 'replace-source', cell: first, source: ['foo\n', 'bar'] }
    );
  });

  test('joins multiline source without changing it', () => {
    const notebook = fixture();

    expect(sourceText(notebook.cells[0].source)).toBe('foo\nbar');
    expect(notebook.cells[0].source).toEqual(['foo\n', 'bar']);
  });

  test('leaves future cells editable', () => {
    const future: UnknownCell & Record<string, unknown> = {
      cell_type: 'future_cell',
      future_extension: { foo: true },
      metadata: {},
    };

    expect(isCodeCell(future)).toBe(false);
    expect(sourceText(future.source)).toBe('');
  });
});

describe('notebook document history', () => {
  test('groups operations into one revision and undo entry', () => {
    const session = open();
    const edited = transact(
      session,
      {
        type: 'replace-source',
        cell: identity(session, 0),
        source: 'bar',
      },
      {
        type: 'replace-source',
        cell: identity(session, 1),
        source: 'qux',
      },
      {
        type: 'set-execution-count',
        cell: identity(session, 0),
        executionCount: 1,
      }
    );

    expect(edited.revision).toBe(1);
    expect(edited.undoStack).toHaveLength(1);

    const undone = undo(edited);
    const redone = redo(undone);

    expect(undone.notebook).toEqual(session.notebook);
    expect(undone.cellIdentities).toEqual(session.cellIdentities);
    expect(undone.revision).toBe(0);
    expect(redone.notebook).toEqual(edited.notebook);
    expect(redone.cellIdentities).toEqual(edited.cellIdentities);
    expect(redone.revision).toBe(1);
  });

  test('restores exact inserted and deleted content', () => {
    const session = open();
    const cell = createSessionCell(session, {
      cell_type: 'code',
      custom: { foo: ['bar'] },
      execution_count: 7,
      metadata: { baz: true },
      outputs: [output('qux')],
      source: ['foo\n', 'bar'],
    } as CodeCell);
    const inserted = transact(session, {
      type: 'insert-cell',
      cell,
      after: identity(session, 1),
    });
    const deleted = transact(inserted, {
      type: 'delete-cell',
      cell: cell.identity,
    });

    expect(undo(deleted).notebook).toEqual(inserted.notebook);
    expect(redo(undo(deleted)).notebook).toEqual(deleted.notebook);
  });

  test('invalidates redo after a new transaction', () => {
    const session = open();
    const first = identity(session, 0);
    const edited = transact(session, {
      type: 'replace-source',
      cell: first,
      source: 'bar',
    });
    const branched = transact(undo(edited), {
      type: 'replace-source',
      cell: first,
      source: 'baz',
    });

    expect(branched.redoStack).toEqual([]);
    expect(redo(branched)).toBe(branched);
  });

  test('bounds undo and redo history', () => {
    let session = open({ historyLimit: 2 });
    const first = identity(session, 0);

    for (const source of ['one', 'two', 'three']) {
      session = transact(session, {
        type: 'replace-source',
        cell: first,
        source,
      });
    }

    expect(session.undoStack).toHaveLength(2);

    const oldest = undo(undo(session));

    expect(sourceText(oldest.notebook.cells[0].source)).toBe('one');
    expect(undo(oldest)).toBe(oldest);
    expect(oldest.redoStack).toHaveLength(2);
  });

  test('tracks saved content across newer edits and undo', () => {
    const session = open();
    const first = identity(session, 0);
    const edited = transact(session, {
      type: 'replace-source',
      cell: first,
      source: 'one',
    });
    const saved = markNotebookSaved(edited, edited.revision);
    const newer = transact(saved, {
      type: 'replace-source',
      cell: first,
      source: 'two',
    });

    expect(saved.revision).toBe(saved.savedRevision);
    expect(newer.revision).not.toBe(newer.savedRevision);

    const restored = undo(newer);

    expect(restored.revision).toBe(restored.savedRevision);
    expect(markNotebookSaved(restored, restored.nextRevision)).toBe(restored);
  });
});
