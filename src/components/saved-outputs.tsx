import { OutputView } from '@/components/output-view';
import { renderOutputs } from '@/lib/output';
import type { NotebookOutput } from '@/lib/types';
import { memo } from 'react';

interface SavedOutputsProps {
  outputs: readonly NotebookOutput[];
}

export const SavedOutputs = memo(function SavedOutputs({
  outputs,
}: SavedOutputsProps) {
  return (
    <section
      className='border-t border-zinc-200 bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-950/50'
      aria-label='Cell outputs'
    >
      <div className='divide-y divide-zinc-200 dark:divide-zinc-800'>
        {renderOutputs(outputs).map((output, index) => (
          <OutputView key={index} output={output} />
        ))}
      </div>
    </section>
  );
});
