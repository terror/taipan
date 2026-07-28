import { invoke } from "@tauri-apps/api/core";
import type { NotebookDocument } from "@/lib/notebook-model";

export function openNotebook(path: string): Promise<NotebookDocument> {
  return invoke<NotebookDocument>("open_notebook", { path });
}

export function saveNotebook(path: string, notebook: NotebookDocument): Promise<void> {
  return invoke("save_notebook", { path, notebook });
}
