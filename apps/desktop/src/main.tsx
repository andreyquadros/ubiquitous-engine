import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './index.css';
import { useAppStore } from './lib/store';
import { watchSystemTheme } from './lib/theme';

watchSystemTheme((t) => useAppStore.getState().setTheme(t));

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
