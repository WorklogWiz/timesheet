import { writable } from 'svelte/store';

// App preferences
export interface AppPreferences {
  use24HourFormat: boolean;
}

// Load preferences from localStorage
const loadPreferences = (): AppPreferences => {
  if (typeof window === 'undefined') {
    return { use24HourFormat: false };
  }

  const stored = localStorage.getItem('appPreferences');
  if (stored) {
    try {
      return JSON.parse(stored);
    } catch {
      return { use24HourFormat: false };
    }
  }
  return { use24HourFormat: false };
};

// Save preferences to localStorage
const savePreferences = (prefs: AppPreferences) => {
  if (typeof window !== 'undefined') {
    localStorage.setItem('appPreferences', JSON.stringify(prefs));
  }
};

// Create the store
function createPreferencesStore() {
  const { subscribe, set, update } = writable<AppPreferences>(loadPreferences());

  return {
    subscribe,
    setUse24Hour: (value: boolean) => {
      update(prefs => {
        const newPrefs = { ...prefs, use24HourFormat: value };
        savePreferences(newPrefs);
        return newPrefs;
      });
    },
    reset: () => {
      const defaultPrefs = { use24HourFormat: false };
      savePreferences(defaultPrefs);
      set(defaultPrefs);
    }
  };
}

export const preferences = createPreferencesStore();



