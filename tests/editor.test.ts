import { describe, expect, test } from 'bun:test';

import { cellEditorLanguage, codeCellLanguage } from '../src/lib/editor';

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
