/**
 * Perplexity AI (perplexity.ai) site handler.
 */

import type { SiteHandler } from './types.js';

export const perplexityHandler: SiteHandler = {
  name: 'Perplexity',

  matches(url: string): boolean {
    return url.includes('perplexity.ai');
  },

  findInputs(): HTMLElement[] {
    const selectors = [
      // Perplexity uses textarea for input
      'textarea[placeholder*="Ask"]',
      'textarea[placeholder*="ask"]',
      'textarea[placeholder*="Search"]',
      'textarea[placeholder*="search"]',
      'textarea',
      '[contenteditable="true"]',
    ];

    for (const selector of selectors) {
      const elements = document.querySelectorAll<HTMLElement>(selector);
      if (elements.length > 0) {
        return Array.from(elements);
      }
    }
    return [];
  },

  findSubmitButtons(): HTMLElement[] {
    const selectors = [
      'button[aria-label*="Submit"]',
      'button[aria-label*="Search"]',
      'button[type="submit"]',
      'button:has(svg)',
    ];

    for (const selector of selectors) {
      try {
        const elements = document.querySelectorAll<HTMLElement>(selector);
        if (elements.length > 0) {
          return Array.from(elements);
        }
      } catch {
        continue;
      }
    }
    return [];
  },

  getPromptText(input: HTMLElement): string {
    if (input instanceof HTMLTextAreaElement) {
      return input.value;
    }
    if (input.isContentEditable) {
      return input.textContent || '';
    }
    return input.innerText || input.textContent || '';
  },

  clearInput(input: HTMLElement): void {
    if (input instanceof HTMLTextAreaElement) {
      input.value = '';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    } else if (input.isContentEditable) {
      input.textContent = '';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
  },

  getOverlayContainer(): HTMLElement | null {
    return document.querySelector('main') || document.body;
  },
};
