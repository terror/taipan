import type { Metadata } from './types';

export type CellEditorLanguage = 'markdown' | 'plain-text' | 'python';

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
