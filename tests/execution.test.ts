import { describe, expect, test } from 'bun:test';

import {
  type ActiveExecution,
  type ExecutionEvent,
  type ExecutionMessage,
  applyExecutionEvent,
  beginExecution,
} from '../src/lib/execution';
import {
  commitCellExecution,
  createNotebookSession,
  isCodeCell,
} from '../src/lib/notebook';
import type { CodeCell, Notebook } from '../src/lib/types';

const KERNEL_ID = '00000000-0000-4000-8000-000000000001';
const DOCUMENT_ID = '00000000-0000-4000-8000-000000000002';
const CELL_ID = '00000000-0000-4000-8000-000000000003';
const EXECUTION_ID = '00000000-0000-4000-8000-000000000004';

function begin(): ActiveExecution {
  return beginExecution(KERNEL_ID, DOCUMENT_ID, CELL_ID, EXECUTION_ID);
}

function event(message: ExecutionMessage): ExecutionEvent {
  return {
    cell_id: CELL_ID,
    document_id: DOCUMENT_ID,
    execution_id: EXECUTION_ID,
    kernel_id: KERNEL_ID,
    message,
  };
}

function apply(
  execution: ActiveExecution,
  ...messages: ExecutionMessage[]
): ActiveExecution {
  return messages.reduce((current, message) => {
    const next = applyExecutionEvent(current, event(message));

    if (!next) {
      throw new Error('Execution disappeared');
    }

    return next;
  }, execution);
}

function reply(): ExecutionMessage {
  return {
    type: 'execute_reply',
    ename: null,
    evalue: null,
    execution_count: 7,
    status: 'ok',
    traceback: null,
  };
}

describe('execution lifecycle', () => {
  test('starts running only at busy and completes in either arrival order', () => {
    function check(last: ExecutionMessage[]) {
      const pending = begin();
      const running = apply(pending, {
        type: 'status',
        execution_state: 'busy',
      });

      expect(pending.running).toBe(false);
      expect(running.running).toBe(true);

      const complete = apply(running, ...last);

      expect(complete.replyReceived).toBe(true);
      expect(complete.idleReceived).toBe(true);
      expect(complete.complete).toBe(true);
      expect(complete.executionCount).toBe(7);
    }

    check([reply(), { type: 'status', execution_state: 'idle' }]);
    check([{ type: 'status', execution_state: 'idle' }, reply()]);
  });

  test('normalizes live values, streams, displays, results, and errors', () => {
    const execution = apply(
      begin(),
      { type: 'status', execution_state: 'busy' },
      { type: 'execute_input', code: 'foo', execution_count: 7 },
      { type: 'stream', name: 'stdout', text: 'foo\n' },
      {
        type: 'display_data',
        data: { 'text/html': '<b>foo</b>' },
        metadata: { foo: true },
      },
      {
        type: 'execute_result',
        data: { 'text/plain': '42' },
        execution_count: 7,
        metadata: {},
      },
      {
        type: 'error',
        ename: 'FooError',
        evalue: 'bar',
        traceback: ['baz'],
      }
    );

    expect(execution.executionCount).toBe(7);
    expect(execution.outputs).toEqual([
      { name: 'stdout', output_type: 'stream', text: 'foo\n' },
      {
        data: { 'text/html': '<b>foo</b>' },
        metadata: { foo: true },
        output_type: 'display_data',
      },
      {
        data: { 'text/plain': '42' },
        execution_count: 7,
        metadata: {},
        output_type: 'execute_result',
      },
      {
        ename: 'FooError',
        evalue: 'bar',
        output_type: 'error',
        traceback: ['baz'],
      },
    ]);
  });

  test('ignores stale opaque identities', () => {
    const execution = begin();
    const stale = {
      ...event({ type: 'status', execution_state: 'busy' }),
      cell_id: '00000000-0000-4000-8000-000000000005',
    };

    expect(applyExecutionEvent(execution, stale)).toBe(execution);
  });

  test('commits completed output and count as one transaction', () => {
    const notebook: Notebook = {
      cells: [
        {
          cell_type: 'code',
          execution_count: 1,
          metadata: {},
          outputs: [{ name: 'stdout', output_type: 'stream', text: 'old\n' }],
          source: 'foo',
        },
      ],
      metadata: {},
      nbformat: 4,
      nbformat_minor: 5,
    };
    const identities = [CELL_ID, DOCUMENT_ID];
    const session = createNotebookSession('foo.ipynb', notebook, {
      generateIdentity: () => identities.shift() ?? EXECUTION_ID,
    });
    const execution = apply(
      beginExecution(KERNEL_ID, session.documentId, CELL_ID, EXECUTION_ID),
      { type: 'status', execution_state: 'busy' },
      { type: 'stream', name: 'stdout', text: 'new\n' },
      reply(),
      { type: 'status', execution_state: 'idle' }
    );
    const executed = commitCellExecution(
      session,
      CELL_ID,
      execution.outputs,
      execution.executionCount
    );
    const cell = executed.notebook.cells[0];

    if (!isCodeCell(cell)) {
      throw new Error('Expected code cell');
    }

    expect(executed.revision).toBe(1);
    expect(executed.revision).not.toBe(executed.savedRevision);
    expect(cell.outputs).toEqual(execution.outputs);
    expect(cell.execution_count).toBe(7);
  });
});
