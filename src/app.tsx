import { CellEditor } from '@/components/cell-editor';
import { KernelSelector } from '@/components/kernel-selector';
import { MarkdownCellView } from '@/components/markdown-cell';
import { SavedOutputs } from '@/components/saved-outputs';
import { Button } from '@/components/ui/button';
import { cellEditorLanguage } from '@/lib/editor';
import {
  type ActiveExecution,
  type KernelSelection,
  applyExecutionEvent,
  beginExecution,
  executeCell,
  listenForExecutionEvents,
  listenForKernelStatus,
} from '@/lib/execution';
import {
  type NotebookSession,
  commitCellExecution,
  createNotebookSession,
  isCodeCell,
  isMarkdownCell,
  markNotebookSaved,
  openNotebook,
  replaceCellSource,
  saveNotebook,
  sessionCells,
  sourceText,
} from '@/lib/notebook';
import { confirm, open } from '@tauri-apps/plugin-dialog';
import { useEffect, useState } from 'react';

function fileName(path: string): string {
  return path.split(/[\\/]/).at(-1) ?? path;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function App() {
  const [session, setSession] = useState<NotebookSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isOpening, setIsOpening] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [kernel, setKernel] = useState<KernelSelection | null>(null);
  const [execution, setExecution] = useState<ActiveExecution | null>(null);

  useEffect(() => {
    let disposed = false;
    let removeExecutionListener: (() => void) | undefined;
    let removeStatusListener: (() => void) | undefined;

    void listenForExecutionEvents((event) =>
      setExecution((current) => applyExecutionEvent(current, event))
    )
      .then((remove) => {
        if (disposed) {
          remove();
        } else {
          removeExecutionListener = remove;
        }
      })
      .catch((cause: unknown) => {
        if (!disposed) {
          setError(errorMessage(cause));
        }
      });

    void listenForKernelStatus((event) =>
      setKernel((current) =>
        current?.kernel_id === event.kernel_id ? event : current
      )
    )
      .then((remove) => {
        if (disposed) {
          remove();
        } else {
          removeStatusListener = remove;
        }
      })
      .catch((cause: unknown) => {
        if (!disposed) {
          setError(errorMessage(cause));
        }
      });

    return () => {
      disposed = true;
      removeExecutionListener?.();
      removeStatusListener?.();
    };
  }, []);

  useEffect(() => {
    if (!execution?.complete) {
      return;
    }

    setSession((current) =>
      current?.documentId === execution.documentId
        ? commitCellExecution(
            current,
            execution.cellId,
            execution.outputs,
            execution.executionCount
          )
        : current
    );
    setExecution((current) =>
      current?.executionId === execution.executionId ? null : current
    );
  }, [execution]);

  async function chooseNotebook() {
    if (session && session.revision !== session.savedRevision) {
      const discard = await confirm(
        'Discard the unsaved changes in this notebook?',
        {
          title: 'Open another notebook',
          kind: 'warning',
        }
      );

      if (!discard) {
        return;
      }
    }

    const path = await open({
      title: 'Open notebook',
      multiple: false,
      directory: false,
      filters: [{ name: 'Jupyter notebooks', extensions: ['ipynb'] }],
    });

    if (!path) {
      return;
    }

    setIsOpening(true);
    setError(null);

    try {
      const notebook = await openNotebook(path);
      setExecution(null);
      setKernel(null);
      setSession(createNotebookSession(path, notebook));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsOpening(false);
    }
  }

  async function saveCurrentNotebook() {
    if (!session || isSaving || session.revision === session.savedRevision) {
      return;
    }

    const revision = session.revision;
    const documentId = session.documentId;
    setIsSaving(true);
    setError(null);

    try {
      await saveNotebook(session.path, session.notebook);
      setSession((current) =>
        current?.documentId === documentId
          ? markNotebookSaved(current, revision)
          : current
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSaving(false);
    }
  }

  async function runCell(cellId: string, code: string) {
    if (!session || !kernel || kernel.state !== 'idle' || execution) {
      return;
    }

    const request = beginExecution(
      kernel.kernel_id,
      session.documentId,
      cellId
    );

    setExecution(request);
    setError(null);

    try {
      await executeCell(request, code);
    } catch (cause) {
      setExecution((current) =>
        current?.executionId === request.executionId ? null : current
      );
      setError(errorMessage(cause));
    }
  }

  const dirty = session ? session.revision !== session.savedRevision : false;

  return (
    <main className='flex h-svh flex-col overflow-hidden bg-zinc-50 font-sans text-zinc-900 antialiased select-none dark:bg-zinc-950 dark:text-zinc-100'>
      {session ? (
        <div className='flex min-h-0 flex-1'>
          <aside className='hidden w-56 shrink-0 border-r border-zinc-200 bg-zinc-100 lg:flex lg:flex-col dark:border-zinc-800 dark:bg-zinc-900'>
            <div className='border-b border-zinc-200 px-4 py-4 dark:border-zinc-800'>
              <p className='text-[10px] font-semibold tracking-[0.13em] text-zinc-500 uppercase dark:text-zinc-400'>
                Document
              </p>
              <p
                className='mt-2 truncate text-[13px] font-medium'
                title={session.path}
              >
                {fileName(session.path)}
              </p>
              <p
                className='mt-0.5 truncate text-[11px] text-zinc-500 dark:text-zinc-400'
                title={session.path}
              >
                {session.path}
              </p>
            </div>
            <div className='px-2 py-3'>
              <div className='flex items-center justify-between rounded-md bg-zinc-200/70 px-2.5 py-2 text-xs dark:bg-zinc-800/80'>
                <span className='font-medium'>Cells</span>
                <span className='text-zinc-500 tabular-nums dark:text-zinc-400'>
                  {session.notebook.cells.length}
                </span>
              </div>
            </div>
            <div className='mt-auto border-t border-zinc-200 px-4 py-3 text-[11px] text-zinc-500 dark:border-zinc-800 dark:text-zinc-400'>
              nbformat {session.notebook.nbformat}.
              {session.notebook.nbformat_minor}
            </div>
          </aside>

          <section className='min-w-0 flex-1 overflow-y-auto bg-zinc-50 dark:bg-zinc-950'>
            <div className='mx-auto w-full max-w-4xl px-4 py-8 sm:px-8 sm:py-12'>
              <div className='mb-8 flex items-end justify-between gap-4 sm:mb-10'>
                <div className='min-w-0'>
                  <p className='text-[10px] font-semibold tracking-[0.13em] text-zinc-500 uppercase dark:text-zinc-400'>
                    Notebook
                  </p>
                  <div className='mt-1.5 flex items-center gap-2'>
                    <h1 className='truncate text-xl font-semibold tracking-[-0.025em] sm:text-2xl'>
                      {fileName(session.path).replace(/\.ipynb$/i, '')}
                    </h1>
                    {dirty && (
                      <span
                        className='size-1.5 shrink-0 rounded-full bg-zinc-600 dark:bg-zinc-400'
                        title='Unsaved changes'
                      />
                    )}
                  </div>
                </div>
                <div className='flex shrink-0 items-center gap-1.5'>
                  <p className='mr-2 hidden text-xs text-zinc-500 tabular-nums sm:block dark:text-zinc-400'>
                    {session.notebook.cells.length}{' '}
                    {session.notebook.cells.length === 1 ? 'cell' : 'cells'}
                  </p>
                  <KernelSelector
                    key={session.documentId}
                    onSelection={setKernel}
                  />
                  <span
                    className='hidden text-[10px] font-semibold tracking-[0.1em] text-zinc-500 uppercase sm:inline dark:text-zinc-400'
                    role='status'
                  >
                    {kernel?.state ?? 'no kernel'}
                  </span>
                  <Button
                    variant='ghost'
                    size='sm'
                    type='button'
                    onClick={() => void chooseNotebook()}
                  >
                    Open
                  </Button>
                  <Button
                    variant='outline'
                    size='sm'
                    type='button'
                    onClick={() => void saveCurrentNotebook()}
                    disabled={!dirty || isSaving}
                  >
                    {isSaving ? 'Saving' : 'Save'}
                  </Button>
                </div>
              </div>

              <div className='space-y-4'>
                {sessionCells(session).map(({ identity, cell }, index) => {
                  const cellExecution =
                    execution?.cellId === identity ? execution : null;
                  const outputs =
                    cellExecution?.outputs ??
                    (isCodeCell(cell) ? cell.outputs : []);

                  return (
                    <article
                      className='group overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-[0_1px_2px_rgba(0,0,0,0.025)] transition-[border-color,box-shadow] duration-150 focus-within:border-zinc-400 focus-within:shadow-[0_0_0_3px_rgba(0,0,0,0.04)] dark:border-zinc-800 dark:bg-zinc-900 dark:focus-within:border-zinc-600 dark:focus-within:shadow-[0_0_0_3px_rgba(255,255,255,0.04)]'
                      key={identity}
                    >
                      <div className='flex items-center justify-between gap-3 px-3 py-2 sm:px-4'>
                        <span className='text-[10px] font-semibold tracking-[0.12em] text-zinc-500 uppercase dark:text-zinc-400'>
                          {cell.cell_type}
                        </span>
                        <div className='flex items-center gap-2'>
                          {isCodeCell(cell) && (
                            <>
                              <span className='font-mono text-[10px] text-zinc-400 tabular-nums dark:text-zinc-500'>
                                In [
                                {cellExecution?.running
                                  ? '*'
                                  : (cell.execution_count ?? ' ')}
                                ]
                              </span>
                              <Button
                                variant='ghost'
                                size='compact'
                                type='button'
                                disabled={
                                  !!execution ||
                                  !kernel ||
                                  kernel.state !== 'idle'
                                }
                                onClick={() =>
                                  void runCell(
                                    identity,
                                    sourceText(cell.source)
                                  )
                                }
                              >
                                {cellExecution ? 'Running...' : 'Run'}
                              </Button>
                            </>
                          )}
                          <span className='font-mono text-[10px] text-zinc-400 tabular-nums dark:text-zinc-500'>
                            {String(index + 1).padStart(2, '0')}
                          </span>
                        </div>
                      </div>
                      {isMarkdownCell(cell) ? (
                        <MarkdownCellView
                          cell={cell}
                          index={index}
                          source={sourceText(cell.source)}
                          onChange={(source) =>
                            setSession((current) =>
                              current
                                ? replaceCellSource(current, identity, source)
                                : current
                            )
                          }
                        />
                      ) : (
                        <CellEditor
                          ariaLabel={`${cell.cell_type} cell ${index + 1}`}
                          language={cellEditorLanguage(
                            cell.cell_type,
                            session.notebook.metadata
                          )}
                          source={sourceText(cell.source)}
                          onChange={(source) =>
                            setSession((current) =>
                              current
                                ? replaceCellSource(current, identity, source)
                                : current
                            )
                          }
                        />
                      )}
                      {isCodeCell(cell) && outputs.length > 0 && (
                        <SavedOutputs
                          outputs={outputs}
                          live={cellExecution !== null}
                        />
                      )}
                    </article>
                  );
                })}
              </div>

              {error && (
                <div
                  className='mt-6 rounded-lg border border-zinc-300 bg-zinc-100 p-3 text-[12px] leading-5 text-zinc-900 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100'
                  role='alert'
                >
                  {error}
                </div>
              )}
            </div>
          </section>
        </div>
      ) : (
        <section className='grid min-h-0 flex-1 place-items-center bg-zinc-50 px-6 py-16 dark:bg-zinc-950'>
          <div className='w-full max-w-sm text-center'>
            <h2 className='text-xl font-semibold tracking-[-0.025em]'>
              Open a notebook
            </h2>
            <p className='mx-auto mt-2 max-w-xs text-[13px] leading-5 text-zinc-500 dark:text-zinc-400'>
              Work with Jupyter notebooks as native documents. Your metadata and
              outputs remain intact.
            </p>
            <Button
              className='mt-6'
              type='button'
              onClick={() => void chooseNotebook()}
              disabled={isOpening}
            >
              {isOpening ? 'Opening...' : 'Choose notebook'}
            </Button>
            {error && (
              <div
                className='mt-6 rounded-lg border border-zinc-300 bg-zinc-100 p-3 text-left text-[12px] leading-5 text-zinc-900 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100'
                role='alert'
              >
                {error}
              </div>
            )}
          </div>
        </section>
      )}
    </main>
  );
}
