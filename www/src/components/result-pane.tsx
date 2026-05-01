import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { usePersistedState } from '@/hooks/use-persisted-state';
import { AlertTriangle, Binary, CheckCircle2, Terminal } from 'lucide-react';

const RESULT_TAB_STORAGE_KEY = 'taipan:result-tab';

type ResultTab = 'bytecode' | 'output';

interface ResultTabState {
  tab: ResultTab;
}

interface ResultPaneProps {
  bytecode: string;
  output: string;
  error: string | undefined;
}

export const ResultPane = ({ bytecode, output, error }: ResultPaneProps) => {
  const [state, setState] = usePersistedState<ResultTabState>(
    RESULT_TAB_STORAGE_KEY,
    { tab: 'output' }
  );

  return (
    <Tabs
      value={state.tab}
      onValueChange={(value) => setState({ tab: value as ResultTab })}
      className='flex h-full min-h-0 flex-col gap-0'
    >
      <div className='bg-panel flex h-11 items-center justify-between border-b px-3'>
        <div className='flex min-w-0 items-center gap-2'>
          {error ? (
            <AlertTriangle className='text-destructive h-4 w-4' />
          ) : (
            <CheckCircle2 className='text-run h-4 w-4' />
          )}
        </div>
        <TabsList>
          <TabsTrigger value='output' className='cursor-pointer'>
            <Terminal className='mr-1 h-3.5 w-3.5' />
            Output
          </TabsTrigger>
          <TabsTrigger value='bytecode' className='cursor-pointer'>
            <Binary className='mr-1 h-3.5 w-3.5' />
            Bytecode
          </TabsTrigger>
        </TabsList>
      </div>

      <TabsContent
        value='output'
        className='bg-editor-background mt-0 min-h-0 flex-1 overflow-hidden'
      >
        <Preformatted
          value={error ?? (output.length === 0 ? 'no output' : output)}
          tone={error ? 'error' : output.length === 0 ? 'muted' : 'code'}
        />
      </TabsContent>
      <TabsContent
        value='bytecode'
        className='bg-editor-background mt-0 min-h-0 flex-1 overflow-hidden'
      >
        <Preformatted
          value={error ?? bytecode}
          tone={error ? 'error' : 'code'}
        />
      </TabsContent>
    </Tabs>
  );
};

function Preformatted({
  value,
  tone,
}: {
  value: string;
  tone: 'code' | 'error' | 'muted';
}) {
  return (
    <pre
      className={[
        'bg-editor-background h-full overflow-auto p-4 font-mono text-sm leading-6 whitespace-pre-wrap',
        tone === 'error' ? 'text-destructive' : '',
        tone === 'muted' ? 'text-muted-foreground' : '',
      ].join(' ')}
    >
      {value}
    </pre>
  );
}
