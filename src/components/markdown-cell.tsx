import { RichContent } from '@/components/rich-content';
import { renderMarkdown } from '@/lib/rich-content';
import type { MarkdownCell } from '@/lib/types';
import { useState } from 'react';

interface MarkdownCellViewProps {
  cell: MarkdownCell;
  index: number;
  source: string;
  onChange: (source: string) => void;
}

export function MarkdownCellView({
  cell,
  index,
  source,
  onChange,
}: MarkdownCellViewProps) {
  const [mode, setMode] = useState<'rendered' | 'source'>('rendered');

  return (
    <>
      <div className='flex justify-end border-t border-zinc-200 px-3 py-1.5 dark:border-zinc-800'>
        <div className='flex rounded-md bg-zinc-100 p-0.5 text-[10px] font-medium dark:bg-zinc-800'>
          {(['rendered', 'source'] as const).map((candidate) => (
            <button
              className={`rounded px-2 py-1 capitalize ${
                mode === candidate
                  ? 'bg-white text-zinc-900 shadow-sm dark:bg-zinc-700 dark:text-zinc-100'
                  : 'text-zinc-500 dark:text-zinc-400'
              }`}
              type='button'
              aria-pressed={mode === candidate}
              key={candidate}
              onClick={() => setMode(candidate)}
            >
              {candidate}
            </button>
          ))}
        </div>
      </div>
      {mode === 'rendered' ? (
        <RichContent
          className='min-h-28 px-3 py-3 sm:px-4'
          html={renderMarkdown(source, cell.attachments)}
        />
      ) : (
        <textarea
          className='block min-h-28 w-full resize-y border-t border-zinc-200 bg-transparent px-3 py-3 text-[13px] leading-6 outline-none select-text sm:px-4 dark:border-zinc-800'
          aria-label={`markdown cell ${index + 1}`}
          value={source}
          onChange={(event) => onChange(event.target.value)}
          spellCheck
        />
      )}
    </>
  );
}
