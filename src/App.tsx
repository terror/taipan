import { useState } from "react";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { AlertCircle, Check, FileCode2, FolderOpen, Save } from "lucide-react";
import { Button } from "@/components/ui/button";
import { openNotebook, saveNotebook } from "@/lib/notebook-client";
import {
  createNotebookSession,
  isDirty,
  markSaved,
  outputCount,
  sourceText,
  updateCellSource,
  type NotebookSession,
} from "@/lib/notebook-model";

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
    if (session && isDirty(session)) {
      const discard = await confirm("Discard the unsaved changes in this notebook?", {
        title: "Open another notebook",
        kind: "warning",
      });

      if (!discard) {
        return;
      }
    }

    const path = await open({
      title: "Open notebook",
      multiple: false,
      directory: false,
      filters: [{ name: "Jupyter notebooks", extensions: ["ipynb"] }],
    });

    if (!path) {
      return;
    }

    setIsOpening(true);
    setError(null);

    try {
      const notebook = await openNotebook(path);
      setSession(createNotebookSession(path, notebook));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsOpening(false);
    }
  }

  async function saveCurrentNotebook() {
    if (!session || isSaving || !isDirty(session)) {
      return;
    }

    const revision = session.revision;
    setIsSaving(true);
    setError(null);

    try {
      await saveNotebook(session.path, session.notebook);
      setSession((current) =>
        current?.path === session.path ? markSaved(current, revision) : current,
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSaving(false);
    }
  }

  if (!session) {
    return (
      <main className="grid min-h-svh place-items-center bg-background p-6 text-foreground">
        <section className="w-full max-w-xl border-l-2 border-foreground pl-8">
          <div className="mb-10 flex size-11 items-center justify-center rounded-md bg-foreground text-background">
            <FileCode2 className="size-5" aria-hidden="true" />
          </div>
          <p className="mb-3 font-mono text-xs uppercase tracking-[0.24em] text-muted-foreground">
            Native notebook workspace
          </p>
          <h1 className="text-5xl font-semibold tracking-[-0.055em]">Taipan</h1>
          <p className="mt-5 max-w-md text-lg leading-8 text-muted-foreground">
            Open a Jupyter notebook as a document. Unknown metadata and outputs stay exactly where
            they belong.
          </p>
          <Button className="mt-9" type="button" onClick={() => void chooseNotebook()} disabled={isOpening}>
            <FolderOpen className="size-4" aria-hidden="true" />
            {isOpening ? "Opening..." : "Open notebook"}
          </Button>
          {error && (
            <p className="mt-6 flex items-start gap-2 text-sm text-red-700 dark:text-red-400">
              <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
              {error}
            </p>
          )}
        </section>
      </main>
    );
  }

  const dirty = isDirty(session);

  return (
    <main className="min-h-svh bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
        <div className="mx-auto flex h-14 max-w-5xl items-center gap-3 px-4 sm:px-6">
          <FileCode2 className="size-5 shrink-0" aria-hidden="true" />
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <h1 className="truncate text-sm font-semibold">{fileName(session.path)}</h1>
              {dirty ? (
                <span className="size-2 shrink-0 rounded-full bg-amber-500" title="Unsaved changes" />
              ) : (
                <Check className="size-3.5 shrink-0 text-muted-foreground" aria-label="Saved" />
              )}
            </div>
            <p className="truncate font-mono text-[10px] text-muted-foreground" title={session.path}>
              {session.path}
            </p>
          </div>
          <Button variant="ghost" size="sm" type="button" onClick={() => void chooseNotebook()}>
            <FolderOpen className="size-4" aria-hidden="true" />
            <span className="hidden sm:inline">Open</span>
          </Button>
          <Button
            size="sm"
            type="button"
            onClick={() => void saveCurrentNotebook()}
            disabled={!dirty || isSaving}
          >
            <Save className="size-4" aria-hidden="true" />
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </div>
      </header>

      <section className="mx-auto max-w-5xl px-4 py-8 sm:px-6 sm:py-12">
        <div className="mb-8 flex items-end justify-between border-b pb-3">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.18em] text-muted-foreground">Notebook</p>
            <p className="mt-1 text-sm text-muted-foreground">
              {session.notebook.cells.length} {session.notebook.cells.length === 1 ? "cell" : "cells"}
            </p>
          </div>
          <p className="font-mono text-xs text-muted-foreground">
            nbformat {session.notebook.nbformat}.{session.notebook.nbformat_minor}
          </p>
        </div>

        <div className="space-y-5">
          {session.notebook.cells.map((cell, index) => {
            const outputs = outputCount(cell);

            return (
              <article className="group grid gap-2 sm:grid-cols-[4rem_minmax(0,1fr)]" key={cell.id ?? index}>
                <div className="flex items-center justify-between sm:block sm:pt-3 sm:text-right">
                  <span className="font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
                    {cell.cell_type}
                  </span>
                  <span className="ml-2 font-mono text-[10px] text-muted-foreground sm:ml-0 sm:block">
                    {String(index + 1).padStart(2, "0")}
                  </span>
                </div>
                <div className="overflow-hidden rounded-lg border bg-card shadow-[0_1px_0_rgba(0,0,0,0.03)] transition-colors focus-within:border-foreground/40">
                  <textarea
                    className={`block min-h-28 w-full resize-y bg-transparent px-4 py-3 text-sm leading-6 outline-none ${
                      cell.cell_type === "code" ? "font-mono" : "font-sans"
                    }`}
                    aria-label={`${cell.cell_type} cell ${index + 1}`}
                    value={sourceText(cell.source)}
                    onChange={(event) =>
                      setSession((current) =>
                        current ? updateCellSource(current, index, event.target.value) : current,
                      )
                    }
                    spellCheck={cell.cell_type === "markdown"}
                  />
                  {outputs > 0 && (
                    <div className="border-t bg-muted/45 px-4 py-2 font-mono text-[11px] text-muted-foreground">
                      {outputs} saved {outputs === 1 ? "output" : "outputs"} preserved
                    </div>
                  )}
                </div>
              </article>
            );
          })}
        </div>

        {error && (
          <div className="mt-8 flex items-start gap-2 rounded-md border border-red-300 bg-red-50 p-3 text-sm text-red-800 dark:border-red-900 dark:bg-red-950/30 dark:text-red-300">
            <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
            {error}
          </div>
        )}
      </section>
    </main>
  );
}
