import { Button } from '@/components/ui/button';
import {
  type NotebookSession,
  isCodeCell,
  openNotebook,
  saveNotebook,
  sourceText,
  updateCellSource,
} from '@/lib/notebook';
import { confirm, open } from '@tauri-apps/plugin-dialog';
import { AlertCircle, FileCode2, FolderOpen, Save } from 'lucide-react';
import { useState } from 'react';

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
      setSession({ path, notebook, revision: 0, savedRevision: 0 });
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
    setIsSaving(true);
    setError(null);

    try {
      await saveNotebook(session.path, session.notebook);
      setSession((current) =>
        current?.path === session.path
          ? {
              ...current,
              savedRevision: Math.max(current.savedRevision, revision),
            }
          : current
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSaving(false);
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
                {session.notebook.cells.map((cell, index) => {
                  const outputCount = isCodeCell(cell)
                    ? cell.outputs.length
                    : 0;

                  return (
                    <article
                      className='group overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-[0_1px_2px_rgba(0,0,0,0.025)] transition-[border-color,box-shadow] duration-150 focus-within:border-zinc-400 focus-within:shadow-[0_0_0_3px_rgba(0,0,0,0.04)] dark:border-zinc-800 dark:bg-zinc-900 dark:focus-within:border-zinc-600 dark:focus-within:shadow-[0_0_0_3px_rgba(255,255,255,0.04)]'
                      key={cell.id ?? index}
                    >
                      <div className='flex items-center justify-between px-3 py-2 sm:px-4'>
                        <span className='text-[10px] font-semibold tracking-[0.12em] text-zinc-500 uppercase dark:text-zinc-400'>
                          {cell.cell_type}
                        </span>
                        <span className='font-mono text-[10px] text-zinc-400 tabular-nums dark:text-zinc-500'>
                          {String(index + 1).padStart(2, '0')}
                        </span>
                      </div>
                      <textarea
                        className={`block min-h-28 w-full resize-y border-t border-zinc-200 bg-transparent px-3 py-3 text-[13px] leading-6 outline-none select-text sm:px-4 dark:border-zinc-800 ${
                          cell.cell_type === 'code' ? 'font-mono' : 'font-sans'
                        }`}
                        aria-label={`${cell.cell_type} cell ${index + 1}`}
                        value={sourceText(cell.source)}
                        onChange={(event) =>
                          setSession((current) =>
                            current
                              ? updateCellSource(
                                  current,
                                  index,
                                  event.target.value
                                )
                              : current
                          )
                        }
                        spellCheck={cell.cell_type === 'markdown'}
                      />
                      {outputCount > 0 && (
                        <div className='flex items-center gap-2 border-t border-zinc-200 bg-zinc-50 px-3 py-2 text-[11px] text-zinc-500 sm:px-4 dark:border-zinc-800 dark:bg-zinc-950/50 dark:text-zinc-400'>
                          <span className='size-1.5 rounded-full bg-zinc-400 dark:bg-zinc-600' />
                          {outputCount} saved{' '}
                          {outputCount === 1 ? 'output' : 'outputs'}
                        </div>
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
