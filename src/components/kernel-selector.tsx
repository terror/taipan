import { discoverKernelspecs } from '@/lib/kernelspec';
import type { KernelDiscovery, Metadata } from '@/lib/types';
import { ChevronDown, Cpu } from 'lucide-react';
import { useEffect, useState } from 'react';

interface KernelSelectorProps {
  metadata: Metadata;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function KernelSelector({ metadata }: KernelSelectorProps) {
  const [discovery, setDiscovery] = useState<KernelDiscovery | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState('');

  useEffect(() => {
    let active = true;

    void discoverKernelspecs(metadata)
      .then((result) => {
        if (!active) {
          return;
        }

        setDiscovery(result);
        setSelectedId(result.recommended_id ?? '');
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(errorMessage(cause));
        }
      });

    return () => {
      active = false;
    };
  }, [metadata]);

  const diagnostics = discovery?.diagnostics ?? [];
  const diagnosticText = diagnostics
    .map(
      (diagnostic) =>
        `${diagnostic.source}${diagnostic.name ? ` / ${diagnostic.name}` : ''}: ${diagnostic.message}`
    )
    .join('\n');

  return (
    <div className='flex min-w-0 flex-col items-end gap-1'>
      <div className='flex h-7 min-w-0 items-center rounded-md border border-zinc-200 bg-white shadow-[0_1px_1px_rgba(0,0,0,0.03)] dark:border-zinc-700 dark:bg-zinc-900'>
        <Cpu
          className='ml-2 size-3.5 shrink-0 text-zinc-500 dark:text-zinc-400'
          aria-hidden='true'
        />
        <div className='relative min-w-0'>
          <select
            className='h-6 max-w-40 min-w-0 cursor-pointer appearance-none bg-transparent py-0 pr-7 pl-1.5 text-xs font-medium outline-none disabled:cursor-default sm:max-w-52 dark:bg-zinc-900'
            aria-label='Notebook kernel'
            value={selectedId}
            disabled={!discovery || discovery.kernels.length === 0}
            onChange={(event) => setSelectedId(event.target.value)}
          >
            {!discovery ? (
              <option value=''>Discovering kernels...</option>
            ) : discovery.kernels.length === 0 ? (
              <option value=''>No local kernels</option>
            ) : (
              <>
                <option value=''>No kernel selected</option>
                {discovery.kernels.map((kernel) => (
                  <option key={kernel.id} value={kernel.id}>
                    {kernel.display_name} ({kernel.source})
                  </option>
                ))}
              </>
            )}
          </select>
          <ChevronDown
            className='pointer-events-none absolute top-1/2 right-2 size-3 -translate-y-1/2 text-zinc-500 dark:text-zinc-400'
            aria-hidden='true'
          />
        </div>
      </div>
      {(error || diagnostics.length > 0) && (
        <p
          className='max-w-52 truncate text-[10px] text-amber-700 dark:text-amber-400'
          title={error ?? diagnosticText}
          role='status'
        >
          {error
            ? 'Kernel discovery failed'
            : `${diagnostics.length} invalid kernelspec${diagnostics.length === 1 ? '' : 's'}`}
        </p>
      )}
    </div>
  );
}
