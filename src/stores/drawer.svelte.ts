import { writable } from "svelte/store";

export const drawerOpen = writable(false);

export const drawerStore = {
  open() {
    drawerOpen.set(true);
  },
  close() {
    drawerOpen.set(false);
  },
};
