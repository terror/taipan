import { useEditorSettings } from '@/providers/editor-settings-provider';
import { python } from '@codemirror/lang-python';
import { EditorState, Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { vim } from '@replit/codemirror-vim';
import { useMemo } from 'react';

export function useEditorExtensions(): Extension[] {
  const { settings } = useEditorSettings();

  return useMemo(() => {
    const extensions: Extension[] = [
      EditorState.tabSize.of(settings.tabSize),
      python(),
      EditorView.theme({
        '&': {
          backgroundColor: 'var(--editor-background)',
        },
        '.cm-content': {
          caretColor: 'var(--foreground)',
        },
        '.cm-cursor': {
          borderLeftColor: 'var(--foreground)',
        },
        '.cm-selectionBackground': {
          backgroundColor: 'oklch(0.9 0 0)',
        },
      }),
    ];

    if (settings.keybindings === 'vim') {
      extensions.push(vim());
    }

    if (settings.lineWrapping) {
      extensions.push(EditorView.lineWrapping);
    }

    return extensions;
  }, [settings.keybindings, settings.lineWrapping, settings.tabSize]);
}
