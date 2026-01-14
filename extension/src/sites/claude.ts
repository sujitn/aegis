/**
 * Claude (claude.ai) site handler.
 */

import type { SiteHandler } from './types.js';

export const claudeHandler: SiteHandler = {
  name: 'Claude',

  matches(url: string): boolean {
    return url.includes('claude.ai');
  },

  findInputs(): HTMLElement[] {
    // Claude uses a contenteditable div with ProseMirror
    const selectors = [
      // Primary: ProseMirror editor
      'div.ProseMirror[contenteditable="true"]',
      '[contenteditable="true"].ProseMirror',
      // Fieldset-based input area
      'fieldset div[contenteditable="true"]',
      // Generic contenteditable in main area
      'main div[contenteditable="true"]',
      'div[contenteditable="true"]',
      // Fallback selectors
      '[data-placeholder*="Reply"]',
      '[data-placeholder*="message"]',
      '.ProseMirror',
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
      // Aria labels for send button
      'button[aria-label*="Send"]',
      'button[aria-label*="send"]',
      // Form submit buttons
      'button[type="submit"]',
      // Buttons in fieldset (near input)
      'fieldset button',
      // Button with send icon
      'button:has(svg[viewBox="0 0 24 24"])',
    ];

    for (const selector of selectors) {
      try {
        const elements = document.querySelectorAll<HTMLElement>(selector);
        if (elements.length > 0) {
          return Array.from(elements);
        }
      } catch {
        // :has() might not be supported in all browsers
        continue;
      }
    }

    // Fallback: find buttons near the input area (bottom of page)
    const buttons = document.querySelectorAll<HTMLElement>('button');
    return Array.from(buttons).filter(btn => {
      const rect = btn.getBoundingClientRect();
      // Button should be visible and near bottom of page
      return rect.width > 0 && rect.height > 0 && rect.bottom > window.innerHeight - 200;
    });
  },

  getPromptText(input: HTMLElement): string {
    if (input.isContentEditable) {
      // Get text content, handling ProseMirror paragraphs
      const paragraphs = input.querySelectorAll('p');
      if (paragraphs.length > 0) {
        return Array.from(paragraphs)
          .map(p => p.textContent || '')
          .join('\n');
      }
      return input.textContent || '';
    }
    return '';
  },

  clearInput(input: HTMLElement): void {
    if (input.isContentEditable) {
      // Clear ProseMirror content
      const paragraphs = input.querySelectorAll('p');
      if (paragraphs.length > 0) {
        paragraphs.forEach(p => {
          p.textContent = '';
        });
      } else {
        input.textContent = '';
      }
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
  },

  getOverlayContainer(): HTMLElement | null {
    return document.querySelector('main') || document.body;
  },
};
