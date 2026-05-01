import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { formatBytecode } from '@/lib/utils';
import { compileSource, executeSource } from '@/lib/wasm';
import dedent from 'dedent';
import { Bot } from 'lucide-react';
import { useCallback } from 'react';
import { useDefaultLayout } from 'react-resizable-panels';

import { EditorPane } from './components/editor-pane';
import { ResultPane } from './components/result-pane';
import { useEditorExtensions } from './hooks/use-editor-extensions';
import { usePersistedState } from './hooks/use-persisted-state';

const DEFAULT_SOURCE = dedent`
  def double(x):
    return x * 2

  value = 0

  while value < 4:
    print(double(value))
    value += 1
`;

const EXAMPLES: CodeExample[] = [
  {
    name: 'Functions',
    source: DEFAULT_SOURCE,
  },
  {
    name: 'Arithmetic',
    source: dedent`
      print(1 + 2)
      print(5 - 3)
      print(3 * 4)
      print(7 / 2)
      print(2 ** 10)
    `,
  },
  {
    name: 'Strings',
    source: dedent`
      foo = "foo"
      bar = "bar"

      print(foo + bar)
      print(foo * 3)
      print(f"{foo}-{bar}")
      print(len(foo))
    `,
  },
  {
    name: 'Control flow',
    source: dedent`
      value = 0

      while value < 5:
        if value == 3:
          break

        print(value)
        value += 1
    `,
  },
];

const EDITOR_STORAGE_KEY = 'taipan:editor-code';
const EXAMPLE_STORAGE_KEY = 'taipan:selected-example';
const PANEL_LAYOUT_STORAGE_KEY = 'taipan:panel-layout';
const RESULT_STORAGE_KEY = 'taipan:execution-result';

interface ExecutionResult {
  bytecode: string;
  error: string | undefined;
  output: string;
}

interface EditorState {
  source: string;
}

interface ExampleState {
  name: string;
}

export interface CodeExample {
  name: string;
  source: string;
}

const App = () => {
  const [editor, setEditor] = usePersistedState<EditorState>(
    EDITOR_STORAGE_KEY,
    {
      source: DEFAULT_SOURCE,
    }
  );

  const [example, setExample] = usePersistedState<ExampleState>(
    EXAMPLE_STORAGE_KEY,
    {
      name: EXAMPLES[0].name,
    }
  );

  const [execution, setExecution] = usePersistedState<ExecutionResult>(
    RESULT_STORAGE_KEY,
    {
      bytecode: '',
      error: undefined,
      output: '',
    }
  );

  const extensions = useEditorExtensions();

  const panelLayout = useDefaultLayout({
    id: PANEL_LAYOUT_STORAGE_KEY,
    panelIds: ['editor-panel', 'result-panel'],
  });

  const run = useCallback(async () => {
    try {
      const code = await compileSource(editor.source);

      const execution = await executeSource(editor.source);

      setExecution({
        bytecode: formatBytecode(code),
        error: undefined,
        output: execution.output,
      });
    } catch (error) {
      setExecution({
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }, [editor.source, setExecution]);

  return (
    <div className='bg-background text-foreground flex h-screen max-w-full flex-col'>
      <div className='flex h-14 items-center justify-between px-4'>
        <div className='flex items-center gap-x-2'>
          <Bot className='h-4 w-4' />
          <a href='/' className='font-semibold'>
            taipan
          </a>
        </div>
      </div>

      <div className='min-h-0 flex-1 overflow-hidden p-4'>
        <ResizablePanelGroup
          defaultLayout={panelLayout.defaultLayout}
          onLayoutChanged={panelLayout.onLayoutChanged}
          orientation='horizontal'
          className='bg-background h-full rounded-lg border shadow-sm'
        >
          <ResizablePanel id='editor-panel' defaultSize='52%' minSize='30%'>
            <EditorPane
              examples={EXAMPLES}
              example={example.name}
              value={editor.source}
              onChange={(source) => setEditor({ source })}
              onExampleChange={(name) => setExample({ name })}
              onReset={() => {
                setEditor({ source: DEFAULT_SOURCE });
                setExample({ name: EXAMPLES[0].name });
              }}
              onRun={run}
              extensions={extensions}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />

          <ResizablePanel id='result-panel' defaultSize='48%' minSize='30%'>
            <ResultPane
              bytecode={execution.bytecode}
              output={execution.output}
              error={execution.error}
            />
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
};

export default App;
