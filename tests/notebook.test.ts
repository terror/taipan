import { describe, expect, test } from 'bun:test';

import {
  type NotebookSession,
  commitCellExecution,
  createNotebookSession,
  isCodeCell,
  markNotebookSaved,
  replaceCellSource,
  sessionCells,
  sourceText,
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
        id: 'foo',
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

function open(): NotebookSession {
  return createNotebookSession('foo.ipynb', fixture(), {
    generateIdentity: counter('identity'),
  });
}

function identity(session: NotebookSession, index: number): string {
  return session.cellIdentities[index];
}

function code(session: NotebookSession): CodeCell {
  const cell = session.notebook.cells[0];

  if (!isCodeCell(cell)) {
    throw new Error('Expected code cell');
  }

  return cell;
}

describe('notebook session', () => {
  test('keeps opaque identities outside serialized notebook content', () => {
    const notebook = fixture();
    const session = createNotebookSession('foo.ipynb', notebook, {
      generateIdentity: counter('identity'),
    });

    expect(session.documentId).toBe('identity-2');
    expect(session.cellIdentities).toEqual(['identity-0', 'identity-1']);
    expect(sessionCells(session).map(({ identity }) => identity)).toEqual(
      session.cellIdentities
    );
    expect(session.notebook).toBe(notebook);
    expect(JSON.stringify(session.notebook)).not.toContain('identity-');
  });

  test('replaces source while preserving unrelated notebook content', () => {
    const session = open();
    const savedOutput = code(session).outputs[0] as NotebookOutput & {
      toJSON?: () => never;
    };

    Object.defineProperty(savedOutput, 'toJSON', {
      value: () => {
        throw new Error('serialized output');
      },
    });

    const edited = replaceCellSource(session, identity(session, 0), 'bar');

    expect(sourceText(edited.notebook.cells[0].source)).toBe('bar');
    expect(code(edited).outputs).toBe(code(session).outputs);
    expect(edited.notebook.metadata).toBe(session.notebook.metadata);
    expect('unknown' in edited.notebook && edited.notebook.unknown).toBe(
      'preserved'
    );
    expect(edited.documentId).toBe(session.documentId);
    expect(edited.cellIdentities).toBe(session.cellIdentities);
    expect(edited.revision).toBe(1);
  });

  test('commits execution outputs and count in one revision', () => {
    const session = open();
    const edited = commitCellExecution(
      session,
      identity(session, 0),
      [output('bar')],
      42
    );

    expect(code(edited).outputs).toEqual([output('bar')]);
    expect(code(edited).execution_count).toBe(42);
    expect(edited.revision).toBe(1);
  });

  test('ignores semantic source no-ops and stale identities', () => {
    const session = open();

    expect(replaceCellSource(session, identity(session, 0), 'foo\nbar')).toBe(
      session
    );
    expect(replaceCellSource(session, 'stale', 'bar')).toBe(session);
  });

  test('ignores unchanged execution and stale identities', () => {
    const session = open();

    expect(
      commitCellExecution(session, identity(session, 0), [output('foo')], null)
    ).toBe(session);
    expect(commitCellExecution(session, 'stale', [output('bar')], 42)).toBe(
      session
    );
  });

  test('tracks saved revisions across newer edits', () => {
    const session = open();
    const edited = replaceCellSource(session, identity(session, 0), 'bar');
    const newer = replaceCellSource(edited, identity(session, 0), 'baz');
    const saved = markNotebookSaved(newer, edited.revision);

    expect(saved.savedRevision).toBe(edited.revision);
    expect(saved.revision).toBe(newer.revision);
    expect(saved.revision).not.toBe(saved.savedRevision);
    expect(markNotebookSaved(saved, saved.revision + 1)).toBe(saved);
  });

  test('joins multiline source and tolerates future cell types', () => {
    const future: UnknownCell = {
      cell_type: 'future_cell',
      metadata: {},
    };

    expect(sourceText(fixture().cells[0].source)).toBe('foo\nbar');
    expect(isCodeCell(future)).toBe(false);
    expect(sourceText(future.source)).toBe('');
  });
});
