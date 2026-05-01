import { buttonVariants } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useEditorSettings } from '@/providers/editor-settings-provider';
import { Settings } from 'lucide-react';
import { useState } from 'react';

export const EditorSettingsDialog = () => {
  const { settings, updateSettings } = useEditorSettings();

  const [settingsOpen, setSettingsOpen] = useState<boolean>(false);

  return (
    <>
      <Tooltip>
        <TooltipTrigger
          className={buttonVariants({
            variant: 'ghost',
            size: 'icon',
            className: 'cursor-pointer',
          })}
          onClick={() => setSettingsOpen(true)}
        >
          <Settings />
        </TooltipTrigger>
        <TooltipContent>Editor settings</TooltipContent>
      </Tooltip>

      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className='sm:max-w-[425px]'>
          <DialogHeader>
            <DialogTitle>Settings</DialogTitle>
            <DialogDescription>
              Customize the editor controls and text rendering.
            </DialogDescription>
          </DialogHeader>
          <div className='grid gap-4 py-4'>
            <div className='flex items-center justify-between'>
              <Label className='text-sm font-medium'>Line numbers</Label>
              <Switch
                checked={settings.lineNumbers}
                onCheckedChange={(checked) =>
                  updateSettings({ lineNumbers: checked })
                }
              />
            </div>

            <div className='flex items-center justify-between'>
              <Label className='text-sm font-medium'>Word wrap</Label>
              <Switch
                checked={settings.lineWrapping}
                onCheckedChange={(checked) =>
                  updateSettings({ lineWrapping: checked })
                }
              />
            </div>

            <SettingSelect
              label='Font size'
              value={settings.fontSize.toString()}
              onValueChange={(value) =>
                updateSettings({ fontSize: parseInt(value) })
              }
              values={[
                ['12', '12px'],
                ['14', '14px'],
                ['16', '16px'],
                ['18', '18px'],
              ]}
            />

            <SettingSelect
              label='Keybindings'
              value={settings.keybindings}
              onValueChange={(value) =>
                updateSettings({ keybindings: value as 'default' | 'vim' })
              }
              values={[
                ['default', 'Default'],
                ['vim', 'Vim'],
              ]}
            />

            <SettingSelect
              label='Tab size'
              value={settings.tabSize.toString()}
              onValueChange={(value) =>
                updateSettings({ tabSize: parseInt(value) })
              }
              values={[
                ['2', '2 spaces'],
                ['4', '4 spaces'],
                ['8', '8 spaces'],
              ]}
            />
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
};

function SettingSelect({
  label,
  value,
  values,
  onValueChange,
}: {
  label: string;
  value: string;
  values: [string, string][];
  onValueChange: (value: string) => void;
}) {
  return (
    <div className='flex items-center justify-between'>
      <Label className='text-sm font-medium'>{label}</Label>
      <Select value={value} onValueChange={onValueChange}>
        <SelectTrigger className='w-28'>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {values.map(([value, label]) => (
            <SelectItem key={value} value={value}>
              {label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
