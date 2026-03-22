import { writable, get } from "svelte/store";
import type { ChatMessage, ToolCallRecord } from "../lib/types";

function generateId() {
  return Math.random().toString(36).slice(2);
}

export const messages = writable<ChatMessage[]>([]);
export const isLoading = writable(false);
export const activeToolCall = writable<string | null>(null);
export const streamingText = writable<string>("");

export const chatStore = {
  addUserMessage(content: string): ChatMessage {
    const msg: ChatMessage = {
      id: generateId(),
      role: "user",
      content,
      timestamp: new Date(),
    };
    messages.update((ms) => [...ms, msg]);
    return msg;
  },

  addAssistantMessage(content: string, toolCalls?: ToolCallRecord[], artifactIds?: string[]): ChatMessage {
    const msg: ChatMessage = {
      id: generateId(),
      role: "assistant",
      content,
      tool_calls: toolCalls,
      artifact_ids: artifactIds,
      timestamp: new Date(),
    };
    messages.update((ms) => [...ms, msg]);
    return msg;
  },

  attachArtifactToLastMessage(artifactId: string) {
    messages.update((ms) => {
      if (ms.length === 0) return ms;
      const last = ms[ms.length - 1];
      if (last.role !== "assistant") return ms;
      const updated = {
        ...last,
        artifact_ids: [...(last.artifact_ids ?? []), artifactId],
      };
      return [...ms.slice(0, -1), updated];
    });
  },

  setLoading(loading: boolean) {
    isLoading.set(loading);
  },

  setActiveToolCall(query: string | null) {
    activeToolCall.set(query);
  },

  clear() {
    messages.set([]);
  },

  getMessages() {
    return get(messages);
  },
};
