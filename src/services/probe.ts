import { invoke, isTauri } from '@tauri-apps/api/core';
export interface Bounds { x: number; y: number; width: number; height: number }
export function boundsFor(elements: HTMLElement[]): Bounds[] {
  return elements.map(element => {
    const { x, y, width, height } = element.getBoundingClientRect();
    return { x, y, width, height };
  });
}
export function messageFor(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') return error.message;
  return typeof error === 'string' ? error : 'Native operation failed. Retry or restart the application.';
}
export async function command<T=void>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isTauri()) throw new Error('Native panels require the desktop app. Run npm run tauri dev.');
  return await invoke<T>(name, args);
}
