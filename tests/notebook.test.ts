import { describe, expect, test } from 'bun:test';

import { isCodeCell, sourceText, updateCellSource } from '../src/lib/notebook';
import type { CodeCell, Notebook, UnknownCell } from '../src/lib/types';

type NotebookFixture = Omit<Notebook, 'cells'> & {
  cells: [CodeCell & Record<string, unknown>];
  unknown: string;
};

function notebook(): NotebookFixture {
  return {
    cells: [
      {
        cell_type: 'code',
        execution_count: null,
        metadata: { foo: 'bar' },
        outputs: [
          {
            name: 'stdout',
            output_type: 'stream',
            text: ['foo\n', 'bar'],
          },
          {
            ename: 'Error',
            evalue: 'foo',
            output_type: 'error',
            traceback: ['foo'],
          },
          {
            data: {
              'application/json': { foo: true },
              'application/x-unsupported': 42,
            },
            execution_count: 1,
            metadata: {},
            output_type: 'execute_result',
          },
          {
            data: { 'text/plain': 'foo' },
            metadata: {},
            output_type: 'display_data',
          },
        ],
        source: ['foo\n', 'bar'],
      },
    ],
    metadata: { custom: { foo: true } },
    nbformat: 4,
    nbformat_minor: 5,
    unknown: 'preserved',
  };
}

describe('notebook model', () => {
  test('ignores an unchanged source update', () => {
    const session = {
      path: 'foo.ipynb',
      notebook: notebook(),
      revision: 0,
      savedRevision: 0,
    };

    expect(updateCellSource(session, 0, 'foo\nbar')).toBe(session);
  });

  test('joins multiline source without changing the document', () => {
    const document = notebook();

    expect(sourceText(document.cells[0].source)).toBe('foo\nbar');
    expect(document.cells[0].source).toEqual(['foo\n', 'bar']);
  });

  test('updates only cell source and tracks revisions', () => {
    const document = notebook();

    const session = {
      path: 'foo.ipynb',
      notebook: document,
      revision: 0,
      savedRevision: 0,
    };

    const edited = updateCellSource(session, 0, 'bar');

    expect(edited.notebook.cells[0]).toEqual({
      ...document.cells[0],
      source: 'bar',
    });

    expect(edited.notebook.metadata).toBe(document.metadata);

    expect('unknown' in edited.notebook && edited.notebook.unknown).toBe(
      'preserved'
    );

    expect(isCodeCell(edited.notebook.cells[0])).toBe(true);

    expect(
      isCodeCell(edited.notebook.cells[0])
        ? edited.notebook.cells[0].outputs
        : undefined
    ).toBe(document.cells[0].outputs);

    expect(edited.revision).toBe(1);
    expect(edited.revision !== edited.savedRevision).toBe(true);

    const saved = {
      ...edited,
      savedRevision: Math.max(edited.savedRevision, edited.revision),
    };

    expect(saved.revision !== saved.savedRevision).toBe(false);
  });

  test('recognizes typed code cells and leaves future cells editable', () => {
    const code = notebook().cells[0];

    const future: UnknownCell & Record<string, unknown> = {
      cell_type: 'future_cell',
      future_extension: { foo: true },
      metadata: {},
    };

    expect(isCodeCell(code)).toBe(true);
    expect(isCodeCell(future)).toBe(false);
    expect(sourceText(future.source)).toBe('');
  });
});
