import { invoke } from "@tauri-apps/api/core";
export const api = <T>(name: string, args: Record<string, unknown> = {}) =>
  invoke<T>(name, args);
