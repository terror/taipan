import { describe, expect, test } from "bun:test";
import { sourceText, updateCellSource } from "../src/lib/notebook-model";
import type { NotebookDocument } from "../src/lib/types";

function notebook(): NotebookDocument {
  return {
    cells: [
      {
        cell_type: "code",
        execution_count: null,
        metadata: { foo: "bar" },
        outputs: [],
        source: ["foo\n", "bar"],
      },
    ],
    metadata: { custom: { foo: true } },
    nbformat: 4,
    nbformat_minor: 5,
    unknown: "preserved",
  };
}

describe("notebook model", () => {
  test("joins multiline source without changing the document", () => {
    const document = notebook();

    expect(sourceText(document.cells[0].source)).toBe("foo\nbar");
    expect(document.cells[0].source).toEqual(["foo\n", "bar"]);
  });

  test("updates only cell source and tracks revisions", () => {
    const document = notebook();
    const session = { path: "foo.ipynb", notebook: document, revision: 0, savedRevision: 0 };
    const edited = updateCellSource(session, 0, "bar");

    expect(edited.notebook.cells[0]).toEqual({ ...document.cells[0], source: "bar" });
    expect(edited.notebook.metadata).toBe(document.metadata);
    expect(edited.notebook.unknown).toBe("preserved");
    expect(edited.revision).toBe(1);
    expect(edited.revision !== edited.savedRevision).toBe(true);

    const saved = { ...edited, savedRevision: Math.max(edited.savedRevision, edited.revision) };

    expect(saved.revision !== saved.savedRevision).toBe(false);
  });

  test("ignores an unchanged source update", () => {
    const session = { path: "foo.ipynb", notebook: notebook(), revision: 0, savedRevision: 0 };

    expect(updateCellSource(session, 0, "foo\nbar")).toBe(session);
  });
});
