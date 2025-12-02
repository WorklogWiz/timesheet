import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

// Create a store for the current theme
export const theme = writable<Theme>('light');

// Detect OS theme
export function detectSystemTheme(): Theme {
  if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }
  return 'light';
}

// Initialize theme detection
export function initTheme() {
  // Set initial theme
  const systemTheme = detectSystemTheme();
  theme.set(systemTheme);
  document.documentElement.setAttribute('data-theme', systemTheme);

  // Listen for theme changes
  if (window.matchMedia) {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', (e) => {
      const newTheme = e.matches ? 'dark' : 'light';
      theme.set(newTheme);
      document.documentElement.setAttribute('data-theme', newTheme);
    });
  }
}


