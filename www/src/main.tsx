import { TooltipProvider } from '@/components/ui/tooltip';
import React from 'react';
import ReactDOM from 'react-dom/client';

import App from './App';
import './index.css';
import { EditorSettingsProvider } from './providers/editor-settings-provider';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <EditorSettingsProvider>
      <TooltipProvider>
        <App />
      </TooltipProvider>
    </EditorSettingsProvider>
  </React.StrictMode>
);
