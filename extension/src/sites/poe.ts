/**
 * Poe (poe.com) site handler.
 */

import type { SiteHandler } from './types.js';

export const poeHandler: SiteHandler = {
  name: 'Poe',

  matches(url: string): boolean {
    return url.includes('poe.com');
  },

  findInputs(): HTMLElement[] {
    const selectors = [
      // Poe uses textarea for input
      'textarea[placeholder*="Talk"]',
      'textarea[placeholder*="Message"]',
      'textarea[class*="TextArea"]',
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
      'button[aria-label*="Send"]',
      'button[class*="SendButton"]',
      'button[type="submit"]',
    ];

    for (const selector of selectors) {
      const elements = document.querySelectorAll<HTMLElement>(selector);
      if (elements.length > 0) {
        return Array.from(elements);
      }
    }

    // Fallback: find buttons with send icon
    const buttons = document.querySelectorAll<HTMLElement>('button');
    return Array.from(buttons).filter(btn => {
      const rect = btn.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0 && rect.bottom > window.innerHeight - 200;
    });
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
    return document.querySelector('main') ||
           document.querySelector('[class*="ChatMessagesView"]') ||
           document.body;
  },
};
