import type { CellEditorLanguage } from '@/lib/editor';
import { defaultKeymap, indentWithTab } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { python } from '@codemirror/lang-python';
import {
  HighlightStyle,
  bracketMatching,
  indentOnInput,
  syntaxHighlighting,
} from '@codemirror/language';
import { Compartment, EditorState, type Extension } from '@codemirror/state';
import {
  EditorView,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
} from '@codemirror/view';
import { tags } from '@lezer/highlight';
import { useEffect, useRef } from 'react';

interface CellEditorProps {
  ariaLabel: string;
  language: CellEditorLanguage;
  onChange: (source: string) => void;
  source: string;
}

interface MountedEditor {
  configuration: Compartment;
  view: EditorView;
}

const highlightStyle = HighlightStyle.define([
  {
    tag: [tags.keyword, tags.controlKeyword, tags.operatorKeyword],
    color: 'var(--editor-syntax-keyword)',
  },
  {
    tag: [tags.bool, tags.null, tags.number],
    color: 'var(--editor-syntax-number)',
  },
  {
    tag: [tags.string, tags.regexp, tags.special(tags.string)],
    color: 'var(--editor-syntax-string)',
  },
  { tag: [tags.comment, tags.meta], color: 'var(--editor-syntax-comment)' },
  {
    tag: [tags.typeName, tags.className, tags.heading],
    color: 'var(--editor-syntax-type)',
    fontWeight: '600',
  },
  {
    tag: [tags.function(tags.variableName), tags.labelName],
    color: 'var(--editor-syntax-function)',
  },
  {
    tag: [tags.link, tags.url],
    color: 'var(--editor-syntax-link)',
    textDecoration: 'underline',
  },
  { tag: tags.strong, fontWeight: '700' },
  { tag: tags.emphasis, fontStyle: 'italic' },
]);

const editorTheme = EditorView.theme({
  '&': {
    backgroundColor: 'transparent',
    color: 'inherit',
    fontSize: '13px',
    minHeight: '7rem',
  },
  '&.cm-focused': { outline: 'none' },
  '.cm-content': {
    caretColor: 'currentColor',
    minHeight: '7rem',
    padding: '0.75rem 1rem',
  },
  '.cm-line': { padding: '0' },
  '.cm-scroller': {
    fontFamily: 'inherit',
    lineHeight: '1.5rem',
    overflowX: 'auto',
  },
  '.cm-activeLine': { backgroundColor: 'var(--editor-active-line)' },
  '.cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection':
    {
      backgroundColor: 'var(--editor-selection) !important',
    },
});

const baseExtensions: Extension = [
  highlightSpecialChars(),
  drawSelection(),
  dropCursor(),
  indentOnInput(),
  bracketMatching(),
  highlightActiveLine(),
  syntaxHighlighting(highlightStyle),
  keymap.of([indentWithTab, ...defaultKeymap]),
  EditorView.lineWrapping,
  editorTheme,
];

export function CellEditor({
  ariaLabel,
  language,
  onChange,
  source,
}: CellEditorProps) {
  const mount = useRef<HTMLDivElement>(null);
  const mounted = useRef<MountedEditor>(null);
  const onChangeRef = useRef(onChange);

  onChangeRef.current = onChange;

  useEffect(() => {
    if (!mount.current) {
      return;
    }

    const configuration = new Compartment();
    const view = new EditorView({
      parent: mount.current,
      state: EditorState.create({
        doc: source,
        extensions: [
          baseExtensions,
          configuration.of(editorConfiguration(language, ariaLabel)),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              onChangeRef.current(update.state.doc.toString());
            }
          }),
        ],
      }),
    });

    mounted.current = { configuration, view };

    return () => {
      mounted.current = null;
      view.destroy();
    };
  }, []);

  useEffect(() => {
    const editor = mounted.current;

    if (editor) {
      editor.view.dispatch({
        effects: editor.configuration.reconfigure(
          editorConfiguration(language, ariaLabel)
        ),
      });
    }
  }, [ariaLabel, language]);

  return (
    <div
      className={`cell-editor min-w-0 border-t border-zinc-200 select-text dark:border-zinc-800 ${
        language === 'python' ? 'font-mono' : 'font-sans'
      }`}
      ref={mount}
    />
  );
}

function editorConfiguration(
  language: CellEditorLanguage,
  ariaLabel: string
): Extension {
  return [
    languageExtension(language),
    EditorView.contentAttributes.of({
      'aria-label': ariaLabel,
      autocapitalize: 'off',
      autocomplete: 'off',
      autocorrect: 'off',
      spellcheck: language === 'markdown' ? 'true' : 'false',
    }),
  ];
}

function languageExtension(language: CellEditorLanguage): Extension {
  switch (language) {
    case 'python':
      return python();
    case 'markdown':
      return markdown();
    case 'plain-text':
      return [];
  }
}
