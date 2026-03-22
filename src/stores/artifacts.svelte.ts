import { writable, get } from "svelte/store";
import type { Artifact } from "../lib/types";

export const artifacts = writable<Artifact[]>([]);

export const artifactsStore = {
  add(artifact: Artifact) {
    artifacts.update((list) => [...list, artifact]);
  },
  getById(id: string): Artifact | undefined {
    return get(artifacts).find((a) => a.id === id);
  },
  clear() {
    artifacts.set([]);
  },
};
