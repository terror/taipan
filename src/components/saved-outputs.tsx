import { OutputView } from '@/components/output-view';
import { renderOutputs } from '@/lib/output';
import type { NotebookOutput } from '@/lib/types';
import { memo } from 'react';

interface SavedOutputsProps {
  live?: boolean;
  outputs: readonly NotebookOutput[];
}

export const SavedOutputs = memo(function SavedOutputs({
  live = false,
  outputs,
}: SavedOutputsProps) {
  return (
    <section
      className='border-t border-zinc-200 bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-950/50'
      aria-label='Cell outputs'
    >
      <div className='flex items-center gap-2 border-b border-zinc-200 px-3 py-2 text-[10px] font-semibold tracking-[0.12em] text-zinc-500 uppercase sm:px-4 dark:border-zinc-800 dark:text-zinc-400'>
        <span
          className={`size-1.5 rounded-full ${live ? 'bg-emerald-500' : 'bg-zinc-400 dark:bg-zinc-600'}`}
        />
        {live ? 'Live output' : 'Output'}
      </div>
      <div className='divide-y divide-zinc-200 dark:divide-zinc-800'>
        {renderOutputs(outputs).map((output, index) => (
          <OutputView key={index} output={output} />
        ))}
      </div>
    </section>
  );
});
