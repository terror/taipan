import {
  EditorState,
  type Transaction,
  type TransactionSpec,
} from '@codemirror/state';
import { describe, expect, test } from 'bun:test';

import {
  EditorDocumentController,
  cellEditorLanguage,
  codeCellLanguage,
  externalDocumentUpdate,
  shouldPublishEditorUpdate,
} from '../src/lib/editor';

class TestEditorView {
  destroyed = 0;
  state = EditorState.create({ doc: 'foo' });
  transactions: Transaction[] = [];

  destroy(): void {
    this.destroyed += 1;
  }

  dispatch(spec: TransactionSpec): void {
    const transaction = this.state.update(spec);

    this.state = transaction.state;
    this.transactions.push(transaction);
  }
}

describe('editor document synchronization', () => {
  test('publishes user changes but not external document updates', () => {
    const state = EditorState.create({ doc: 'foo' });
    const user = state.update({ changes: { from: 3, insert: 'bar' } });
    const external = user.state.update({
      annotations: externalDocumentUpdate.of(true),
      changes: { from: 0, to: user.state.doc.length, insert: 'baz' },
    });

    expect(shouldPublishEditorUpdate([user])).toBe(true);
    expect(shouldPublishEditorUpdate([external])).toBe(false);
    expect(shouldPublishEditorUpdate([user, external])).toBe(true);
  });

  test('applies external undo and redo sources without feedback updates', () => {
    const view = new TestEditorView();
    const controller = new EditorDocumentController(view);

    expect(controller.synchronize('foo')).toBe(false);
    expect(controller.synchronize('bar')).toBe(true);
    expect(view.state.doc.toString()).toBe('bar');
    expect(shouldPublishEditorUpdate(view.transactions)).toBe(false);
    expect(controller.synchronize('foo')).toBe(true);
    expect(view.state.doc.toString()).toBe('foo');
    expect(shouldPublishEditorUpdate(view.transactions)).toBe(false);
  });

  test('destroys the editor once and ignores later synchronization', () => {
    const view = new TestEditorView();
    const controller = new EditorDocumentController(view);

    controller.dispose();
    controller.dispose();

    expect(view.destroyed).toBe(1);
    expect(controller.synchronize('bar')).toBe(false);
    expect(view.state.doc.toString()).toBe('foo');
  });
});

describe('editor language selection', () => {
  test('selects Python from standard notebook metadata', () => {
    function check(metadata: Record<string, unknown>) {
      expect(codeCellLanguage(metadata)).toBe('python');
    }

    check({ kernelspec: { language: 'python' } });
    check({ kernelspec: { name: 'python3' } });
    check({ language_info: { name: 'Python 3' } });
  });

  test('uses plain text when metadata does not indicate Python', () => {
    function check(metadata: Record<string, unknown>) {
      expect(codeCellLanguage(metadata)).toBe('plain-text');
    }

    check({});
    check({ kernelspec: { language: 'julia' } });
    check({ kernelspec: null, language_info: [] });
  });

  test('selects Markdown and plain text modes by cell type', () => {
    const metadata = { kernelspec: { language: 'python' } };

    expect(cellEditorLanguage('code', metadata)).toBe('python');
    expect(cellEditorLanguage('markdown', metadata)).toBe('markdown');
    expect(cellEditorLanguage('raw', metadata)).toBe('plain-text');
    expect(cellEditorLanguage('future', metadata)).toBe('plain-text');
  });
});
