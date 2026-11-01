import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.jsx'
import { applyTheme, getStorage, loadTheme } from './lib/theme.js'

// Apply the saved theme before first paint to avoid a flash.
try {
  applyTheme(document.documentElement, loadTheme(getStorage(window)))
} catch {
  /* theme falls back to the OS preference */
}

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
