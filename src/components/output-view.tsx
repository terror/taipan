import { OutputText } from '@/components/output-text';
import type { RenderedOutput } from '@/lib/output';

interface OutputViewProps {
  output: RenderedOutput;
}

export function OutputView({ output }: OutputViewProps) {
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
