/**
 * Microsoft Copilot (copilot.microsoft.com) site handler.
 */

import type { SiteHandler } from './types.js';

export const copilotHandler: SiteHandler = {
  name: 'Copilot',

  matches(url: string): boolean {
    return url.includes('copilot.microsoft.com') || url.includes('bing.com/chat');
  },

  findInputs(): HTMLElement[] {
    const selectors = [
      // Copilot text input
      'textarea[placeholder*="message"]',
      'textarea[placeholder*="Message"]',
      '#searchbox',
      '[contenteditable="true"]',
      'cib-text-input textarea',
      // Bing Chat
      '#b_sydConvCont textarea',
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
      'button[type="submit"]',
      'cib-text-input button',
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
           document.querySelector('#b_sydConvCont') ||
           document.body;
  },
};
