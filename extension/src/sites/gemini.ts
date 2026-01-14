/**
 * Gemini (gemini.google.com) site handler.
 */

import type { SiteHandler } from './types.js';

export const geminiHandler: SiteHandler = {
  name: 'Gemini',

  matches(url: string): boolean {
    return url.includes('gemini.google.com');
  },

  findInputs(): HTMLElement[] {
    // Gemini uses various input elements
    const selectors = [
      'rich-textarea [contenteditable="true"]',
      '.ql-editor[contenteditable="true"]',
      '[contenteditable="true"][aria-label*="Enter"]',
      'textarea[aria-label*="Enter"]',
      '[role="textbox"]',
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
      'button[aria-label*="Send"]',
      '.send-button',
      'button[data-test-id="send-button"]',
    ];

    for (const selector of selectors) {
      const elements = document.querySelectorAll<HTMLElement>(selector);
      if (elements.length > 0) {
        return Array.from(elements);
      }
    }

    // Fallback: find mat-icon buttons that might be send
    const iconButtons = document.querySelectorAll<HTMLElement>('button mat-icon');
    return Array.from(iconButtons)
      .map(icon => icon.closest('button') as HTMLElement)
      .filter((btn): btn is HTMLElement => btn !== null);
  },

  getPromptText(input: HTMLElement): string {
    if (input instanceof HTMLTextAreaElement) {
      return input.value;
    }
    if (input.isContentEditable) {
      return input.textContent || '';
    }
    // For rich text areas, try getting innerText
    return input.innerText || input.textContent || '';
  },

  clearInput(input: HTMLElement): void {
    if (input instanceof HTMLTextAreaElement) {
      input.value = '';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    } else if (input.isContentEditable) {
      input.textContent = '';
      input.innerHTML = '';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
  },

  getOverlayContainer(): HTMLElement | null {
    // Gemini's main content area
    return document.querySelector('main') ||
           document.querySelector('.conversation-container') ||
           document.body;
  },
};
