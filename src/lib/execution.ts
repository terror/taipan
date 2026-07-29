import { invoke } from '@tauri-apps/api/core';
import { type UnlistenFn, listen } from '@tauri-apps/api/event';

import type { ExecutionCount, MimeBundle, NotebookOutput } from './types';

export type KernelId = string;
export type ExecutionId = string;

export type KernelState =
  | 'busy'
  | 'exited'
  | 'failed'
  | 'idle'
  | 'starting'
  | 'stopping'
  | 'unresponsive';

export interface KernelSelection {
  kernel_id: KernelId;
  state: KernelState;
}

export interface KernelStatusEvent extends KernelSelection {}

export type ExecutionMessage =
  | {
      type: 'display_data';
      data: MimeBundle;
      metadata: Record<string, unknown>;
    }
  | {
      type: 'error';
      ename: string;
      evalue: string;
      traceback: string[];
    }
  | {
      type: 'execute_input';
      code: string;
      execution_count: number;
    }
  | {
      type: 'execute_reply';
      ename: string | null;
      evalue: string | null;
      execution_count: number;
      status: string;
      traceback: string[] | null;
    }
  | {
      type: 'execute_result';
      data: MimeBundle;
      execution_count: number;
      metadata: Record<string, unknown>;
    }
  | {
      type: 'status';
      execution_state: 'busy' | 'idle';
    }
  | {
      type: 'stream';
      name: string;
      text: string;
    };

export interface ExecutionEvent {
  cell_id: string;
  document_id: string;
  execution_id: ExecutionId;
  kernel_id: KernelId;
  message: ExecutionMessage;
}

export interface ActiveExecution {
  cellId: string;
  complete: boolean;
  documentId: string;
  executionCount: ExecutionCount;
  executionId: ExecutionId;
  idleReceived: boolean;
  kernelId: KernelId;
  outputs: NotebookOutput[];
  replyReceived: boolean;
  running: boolean;
}

export function beginExecution(
  kernelId: KernelId,
  documentId: string,
  cellId: string,
  executionId = crypto.randomUUID()
): ActiveExecution {
  return {
    cellId,
    complete: false,
    documentId,
    executionCount: null,
    executionId,
    idleReceived: false,
    kernelId,
    outputs: [],
    replyReceived: false,
    running: false,
  };
}

export function applyExecutionEvent(
  execution: ActiveExecution | null,
  event: ExecutionEvent
): ActiveExecution | null {
  if (!execution || !matches(execution, event) || execution.complete) {
    return execution;
  }

  const message = event.message;
  let next = execution;

  switch (message.type) {
    case 'display_data':
      next = {
        ...execution,
        outputs: [
          ...execution.outputs,
          {
            data: message.data,
            metadata: message.metadata,
            output_type: 'display_data',
          },
        ],
      };
      break;
    case 'error':
      next = {
        ...execution,
        outputs: [
          ...execution.outputs,
          {
            ename: message.ename,
            evalue: message.evalue,
            output_type: 'error',
            traceback: message.traceback,
          },
        ],
      };
      break;
    case 'execute_input':
      next = { ...execution, executionCount: message.execution_count };
      break;
    case 'execute_reply':
      next = {
        ...execution,
        executionCount: message.execution_count,
        replyReceived: true,
      };
      break;
    case 'execute_result':
      next = {
        ...execution,
        executionCount: message.execution_count,
        outputs: [
          ...execution.outputs,
          {
            data: message.data,
            execution_count: message.execution_count,
            metadata: message.metadata,
            output_type: 'execute_result',
          },
        ],
      };
      break;
    case 'status':
      next =
        message.execution_state === 'busy'
          ? { ...execution, running: true }
          : execution.running
            ? { ...execution, idleReceived: true }
            : execution;
      break;
    case 'stream':
      next = {
        ...execution,
        outputs: [
          ...execution.outputs,
          {
            name: message.name,
            output_type: 'stream',
            text: message.text,
          },
        ],
      };
      break;
  }

  return {
    ...next,
    complete: next.running && next.replyReceived && next.idleReceived,
  };
}

export function executeCell(
  execution: ActiveExecution,
  code: string
): Promise<void> {
  return invoke('execute_cell', {
    cellId: execution.cellId,
    code,
    documentId: execution.documentId,
    executionId: execution.executionId,
    kernelId: execution.kernelId,
  });
}

export function listenForExecutionEvents(
  handler: (event: ExecutionEvent) => void
): Promise<UnlistenFn> {
  return listen<ExecutionEvent>('execution-message', (event) =>
    handler(event.payload)
  );
}

export function listenForKernelStatus(
  handler: (event: KernelStatusEvent) => void
): Promise<UnlistenFn> {
  return listen<KernelStatusEvent>('kernel-status', (event) =>
    handler(event.payload)
  );
}

function matches(execution: ActiveExecution, event: ExecutionEvent): boolean {
  return (
    execution.cellId === event.cell_id &&
    execution.documentId === event.document_id &&
    execution.executionId === event.execution_id &&
    execution.kernelId === event.kernel_id
  );
}
