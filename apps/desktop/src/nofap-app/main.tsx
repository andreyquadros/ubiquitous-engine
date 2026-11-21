import React from 'react';
import ReactDOM from 'react-dom/client';
// Self-hosted variable fonts (the Tauri CSP allows only `self`; never load fonts from Google).
import '@fontsource-variable/inter';
import '@fontsource-variable/sora';
import AppNofap from './AppNofap';
import '../index.css';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <AppNofap />
  </React.StrictMode>,
);
