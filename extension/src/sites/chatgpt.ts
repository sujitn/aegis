/**
 * ChatGPT (chat.openai.com, chatgpt.com) site handler.
 */

import type { SiteHandler } from './types.js';

export const chatgptHandler: SiteHandler = {
  name: 'ChatGPT',

  matches(url: string): boolean {
    return url.includes('chat.openai.com') || url.includes('chatgpt.com');
  },

  findInputs(): HTMLElement[] {
    // ChatGPT uses a contenteditable div or textarea
    const selectors = [
      '#prompt-textarea',
      'textarea[data-id="root"]',
      '[contenteditable="true"][data-testid]',
      'div[contenteditable="true"]',
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
      'button[data-testid="send-button"]',
      'button[data-testid="fruitjuice-send-button"]',
      'form button[type="submit"]',
      'button[aria-label*="Send"]',
    ];

    for (const selector of selectors) {
      const elements = document.querySelectorAll<HTMLElement>(selector);
      if (elements.length > 0) {
        return Array.from(elements);
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
    return '';
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
    // Find the main chat container
    return document.querySelector('main') || document.body;
  },
};
