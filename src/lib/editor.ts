import {
  Annotation,
  type EditorState,
  type Transaction,
  type TransactionSpec,
} from '@codemirror/state';

import type { Metadata } from './types';

export type CellEditorLanguage = 'markdown' | 'plain-text' | 'python';

export const externalDocumentUpdate = Annotation.define<boolean>();

interface ControlledEditorView {
  readonly state: EditorState;
  destroy(): void;
  dispatch(spec: TransactionSpec): void;
}

export class EditorDocumentController {
  private disposed = false;

  constructor(private readonly view: ControlledEditorView) {}

  synchronize(source: string): boolean {
    if (this.disposed || this.view.state.doc.toString() === source) {
      return false;
    }

    this.view.dispatch({
      annotations: externalDocumentUpdate.of(true),
      changes: {
        from: 0,
        to: this.view.state.doc.length,
        insert: source,
      },
    });

    return true;
  }

  dispose(): void {
    if (this.disposed) {
      return;
    }

    this.disposed = true;
    this.view.destroy();
  }
}

export function shouldPublishEditorUpdate(
  transactions: readonly Transaction[]
): boolean {
  return transactions.some(
    (transaction) =>
      transaction.docChanged &&
      transaction.annotation(externalDocumentUpdate) !== true
  );
}

export function codeCellLanguage(metadata: Metadata): CellEditorLanguage {
  const kernelspec = record(metadata.kernelspec);
  const languageInfo = record(metadata.language_info);
  const candidates = [
    kernelspec?.language,
    kernelspec?.name,
    languageInfo?.name,
  ];

  return candidates.some(
    (candidate) =>
      typeof candidate === 'string' &&
      candidate.trim().toLowerCase().startsWith('python')
  )
    ? 'python'
    : 'plain-text';
}

export function cellEditorLanguage(
  cellType: string,
  metadata: Metadata
): CellEditorLanguage {
  if (cellType === 'markdown') {
    return 'markdown';
  }

  return cellType === 'code' ? codeCellLanguage(metadata) : 'plain-text';
}

function record(value: unknown): Record<string, unknown> | undefined {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}
