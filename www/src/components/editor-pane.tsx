import { buttonVariants } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Extension } from '@codemirror/state';
import { Play, RotateCcw } from 'lucide-react';

import type { CodeExample } from '../App';
import { Editor } from './editor';
import { EditorSettingsDialog } from './editor-settings-dialog';

interface EditorPaneProps {
  examples: CodeExample[];
  example: string;
  value: string;
  onChange: (value: string) => void;
  onExampleChange: (name: string) => void;
  onReset: () => void;
  onRun: () => void;
  extensions: Extension[];
}

export const EditorPane = ({
  examples,
  example,
  value,
  onChange,
  onExampleChange,
  onReset,
  onRun,
  extensions,
}: EditorPaneProps) => {
  return (
    <div className='flex h-full min-h-0 flex-col overflow-hidden'>
      <div className='bg-panel flex h-11 items-center justify-between border-b px-3'>
        <Select
          value={example}
          onValueChange={(name) => {
            const example = examples.find((example) => example.name === name);

            if (example) {
              onExampleChange(example.name);
              onChange(example.source);
            }
          }}
        >
          <SelectTrigger className='border-border/70 bg-background hover:bg-background focus-visible:border-border h-7 w-36 cursor-pointer rounded-md border px-2 py-0 text-xs shadow-xs transition-colors focus-visible:ring-0 focus-visible:ring-offset-0'>
            <SelectValue placeholder='Examples' />
          </SelectTrigger>
          <SelectContent
            position='popper'
            side='bottom'
            avoidCollisions={false}
            className='min-w-36'
          >
            {examples.map((example) => (
              <SelectItem
                key={example.name}
                value={example.name}
                className='focus:text-foreground hover:bg-muted/70 hover:text-foreground focus:bg-muted/70 cursor-pointer rounded-md py-1.5 text-xs'
              >
                {example.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <div className='flex items-center gap-1'>
          <EditorSettingsDialog />
          <Tooltip>
            <TooltipTrigger
              className={buttonVariants({
                variant: 'ghost',
                size: 'icon',
                className: 'cursor-pointer',
              })}
              onClick={onReset}
            >
              <RotateCcw />
            </TooltipTrigger>
            <TooltipContent>Reset source</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger
              className={buttonVariants({
                variant: 'default',
                size: 'sm',
                className: 'cursor-pointer',
              })}
              onClick={onRun}
            >
              <Play />
              Run
            </TooltipTrigger>
            <TooltipContent>Compile and execute</TooltipContent>
          </Tooltip>
        </div>
      </div>
      <div className='flex-1 overflow-hidden'>
        <Editor value={value} onChange={onChange} extensions={extensions} />
      </div>
    </div>
  );
};
