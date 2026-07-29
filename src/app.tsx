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
  applyTransaction,
  createNotebookSession,
  isCodeCell,
  isMarkdownCell,
  markNotebookSaved,
  openNotebook,
  saveNotebook,
  sessionCells,
  sourceText,
} from '@/lib/notebook';
import { confirm, open } from '@tauri-apps/plugin-dialog';
import {
  AlertCircle,
  FileCode2,
  FolderOpen,
  LoaderCircle,
  Play,
  Save,
} from 'lucide-react';
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
        ? applyTransaction(current, [
            {
              type: 'replace-outputs',
              cell: execution.cellId,
              outputs: execution.outputs,
            },
            {
              type: 'set-execution-count',
              cell: execution.cellId,
              executionCount: execution.executionCount,
            },
          ])
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
                    <FolderOpen className='size-3.5' aria-hidden='true' />
                    <span className='hidden sm:inline'>Open</span>
                  </Button>
                  <Button
                    variant='outline'
                    size='sm'
                    type='button'
                    onClick={() => void saveCurrentNotebook()}
                    disabled={!dirty || isSaving}
                  >
                    <Save className='size-3.5' aria-hidden='true' />
                    <span className='hidden sm:inline'>
                      {isSaving ? 'Saving' : 'Save'}
                    </span>
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
                      aria-label={`${cell.cell_type} cell ${index + 1}`}
                      key={identity}
                    >
                      <div className='flex items-center justify-between gap-3 px-3 py-2 sm:px-4'>
                        <span className='text-[10px] font-semibold tracking-[0.12em] text-zinc-500 uppercase dark:text-zinc-400'>
                          {cell.cell_type}
                        </span>
                        {isCodeCell(cell) && (
                          <div className='flex items-center gap-2'>
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
                                void runCell(identity, sourceText(cell.source))
                              }
                            >
                              {cellExecution ? (
                                <LoaderCircle
                                  className='size-3 animate-spin'
                                  aria-hidden='true'
                                />
                              ) : (
                                <Play className='size-3' aria-hidden='true' />
                              )}
                              Run
                            </Button>
                          </div>
                        )}
                      </div>
                      {isMarkdownCell(cell) ? (
                        <MarkdownCellView
                          cell={cell}
                          index={index}
                          source={sourceText(cell.source)}
                          onChange={(source) =>
                            setSession((current) =>
                              current
                                ? applyTransaction(current, [
                                    {
                                      type: 'replace-source',
                                      cell: identity,
                                      source,
                                    },
                                  ])
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
                                ? applyTransaction(current, [
                                    {
                                      type: 'replace-source',
                                      cell: identity,
                                      source,
                                    },
                                  ])
                                : current
                            )
                          }
                        />
                      )}
                      {isCodeCell(cell) && outputs.length > 0 && (
                        <SavedOutputs
                          live={cellExecution !== null}
                          outputs={outputs}
                        />
                      )}
                    </article>
                  );
                })}
              </div>

              {error && (
                <div
                  className='mt-6 flex items-start gap-2 rounded-lg border border-zinc-300 bg-zinc-100 p-3 text-[12px] leading-5 text-zinc-900 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100'
                  role='alert'
                >
                  <AlertCircle
                    className='mt-0.5 size-4 shrink-0'
                    aria-hidden='true'
                  />
                  {error}
                </div>
              )}
            </div>
          </section>
        </div>
      ) : (
        <section className='grid min-h-0 flex-1 place-items-center bg-zinc-50 px-6 py-16 dark:bg-zinc-950'>
          <div className='w-full max-w-sm text-center'>
            <div className='mx-auto flex size-14 items-center justify-center rounded-2xl border border-zinc-200 bg-white shadow-sm dark:border-zinc-800 dark:bg-zinc-900'>
              <FileCode2
                className='size-6 text-zinc-500 dark:text-zinc-400'
                strokeWidth={1.5}
                aria-hidden='true'
              />
            </div>
            <h2 className='mt-6 text-xl font-semibold tracking-[-0.025em]'>
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
              <FolderOpen className='size-4' aria-hidden='true' />
              {isOpening ? 'Opening...' : 'Choose notebook'}
            </Button>
            {error && (
              <div
                className='mt-6 flex items-start gap-2 rounded-lg border border-zinc-300 bg-zinc-100 p-3 text-left text-[12px] leading-5 text-zinc-900 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100'
                role='alert'
              >
                <AlertCircle
                  className='mt-0.5 size-4 shrink-0'
                  aria-hidden='true'
                />
                {error}
              </div>
            )}
          </div>
        </section>
      )}
    </main>
  );
}
