import { invoke } from "@tauri-apps/api/core";
import type { Notebook } from "@/lib/types";

export function openNotebook(path: string): Promise<Notebook> {
  return invoke<Notebook>("open_notebook", { path });
}

export function saveNotebook(path: string, notebook: Notebook): Promise<void> {
  return invoke("save_notebook", { path, notebook });
}
