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
    // Claude uses a contenteditable div
    const selectors = [
      '[contenteditable="true"].ProseMirror',
      'div[contenteditable="true"]',
      '[data-placeholder*="Reply"]',
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
      'button[aria-label*="Send"]',
      'button[type="submit"]',
      'button:has(svg[viewBox="0 0 24 24"])', // Send icon button
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

    // Fallback: find buttons near the input
    const buttons = document.querySelectorAll<HTMLElement>('button');
    return Array.from(buttons).filter(btn => {
      const rect = btn.getBoundingClientRect();
      return rect.bottom > window.innerHeight - 200; // Near bottom of page
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
