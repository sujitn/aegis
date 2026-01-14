/**
 * Site handlers registry.
 */

import type { SiteHandler } from './types.js';
import { chatgptHandler } from './chatgpt.js';
import { claudeHandler } from './claude.js';
import { geminiHandler } from './gemini.js';

export type { SiteHandler, InterceptCallback } from './types.js';

const handlers: SiteHandler[] = [
  chatgptHandler,
  claudeHandler,
  geminiHandler,
];

/**
 * Get the appropriate site handler for the current URL.
 */
export function getSiteHandler(url: string = window.location.href): SiteHandler | null {
  for (const handler of handlers) {
    if (handler.matches(url)) {
      return handler;
    }
  }
  return null;
}

/**
 * Get all registered site handlers.
 */
export function getAllHandlers(): SiteHandler[] {
  return [...handlers];
}
