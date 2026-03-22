import { writable } from "svelte/store";

export const settingsOpen = writable(false);

export const settingsStore = {
  open() { settingsOpen.set(true); },
  close() { settingsOpen.set(false); },
};
