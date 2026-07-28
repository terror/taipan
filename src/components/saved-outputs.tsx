import { type RenderedOutput, renderOutputs } from '@/lib/output';
import type { NotebookOutput } from '@/lib/types';

interface SavedOutputsProps {
  outputs: readonly NotebookOutput[];
}

export function SavedOutputs({ outputs }: SavedOutputsProps) {
  return (
    <section
      className='border-t border-zinc-200 bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-950/50'
      aria-label='Saved outputs loaded from disk'
    >
      <div className='flex items-center gap-2 border-b border-zinc-200 px-3 py-2 text-[10px] font-semibold tracking-[0.12em] text-zinc-500 uppercase sm:px-4 dark:border-zinc-800 dark:text-zinc-400'>
        <span className='size-1.5 rounded-full bg-zinc-400 dark:bg-zinc-600' />
        Loaded from disk
      </div>
      <div className='divide-y divide-zinc-200 dark:divide-zinc-800'>
        {renderOutputs(outputs).map((output, index) => (
          <OutputView key={index} output={output} />
        ))}
      </div>
    </section>
  );
}

function OutputView({ output }: { output: RenderedOutput }) {
  return (
    <div
      className='px-3 py-3 sm:px-4'
      data-output-type={output.outputType}
      data-renderer={output.renderer}
      data-truncated={output.truncated}
    >
      {output.renderer === 'stream' && (
        <>
          <p
            className={`mb-1.5 text-[10px] font-semibold tracking-[0.1em] uppercase ${
              output.stream === 'stderr'
                ? 'text-red-600 dark:text-red-400'
                : 'text-zinc-500 dark:text-zinc-400'
            }`}
          >
            {output.stream}
          </p>
          <OutputText text={output.text} error={output.stream === 'stderr'} />
        </>
      )}
      {output.renderer === 'error' && (
        <div className='text-red-700 dark:text-red-300'>
          <p className='font-mono text-[12px] font-semibold select-text'>
            {output.name}
            {output.value && `: ${output.value}`}
          </p>
          {output.traceback && (
            <OutputText text={output.traceback} error className='mt-2' />
          )}
        </div>
      )}
      {output.renderer === 'text/plain' && <OutputText text={output.text} />}
      {output.renderer === 'unsupported' && (
        <div className='rounded-md border border-dashed border-zinc-300 px-3 py-2 text-[11px] leading-5 text-zinc-500 dark:border-zinc-700 dark:text-zinc-400'>
          Unsupported output
          {output.mimeTypes && `: ${output.mimeTypes}`}
        </div>
      )}
      {output.truncated && (
        <p
          className='mt-2 text-[10px] font-medium tracking-wide text-amber-700 dark:text-amber-400'
          role='status'
        >
          Output truncated: {output.omittedCharacters.toLocaleString()}{' '}
          characters not shown
        </p>
      )}
    </div>
  );
}

function OutputText({
  text,
  error = false,
  className = '',
}: {
  text: string;
  error?: boolean;
  className?: string;
}) {
  return (
    <pre
      className={`font-mono text-[12px] leading-5 break-words whitespace-pre-wrap select-text ${
        error
          ? 'text-red-700 dark:text-red-300'
          : 'text-zinc-800 dark:text-zinc-200'
      } ${className}`}
    >
      {text}
    </pre>
  );
}
