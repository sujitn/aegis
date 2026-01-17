/**
 * Network Request Interceptor for Aegis.
 *
 * Intercepts fetch() and XMLHttpRequest to catch API calls to LLM services.
 * This approach is immune to DOM changes since it intercepts the actual API requests.
 */

import { checkPrompt, shouldBlock, shouldWarn, type CheckResponse } from './api.js';

/**
 * API endpoint patterns for LLM services.
 * These are the actual backend APIs, not the UI - much more stable than DOM selectors.
 */
export const API_PATTERNS = {
  chatgpt: [
    /^https:\/\/(chat\.openai\.com|chatgpt\.com)\/backend-api\/conversation$/,
    /^https:\/\/(chat\.openai\.com|chatgpt\.com)\/backend-api\/sentinel\/chat-requirements$/,
  ],
  claude: [
    /^https:\/\/claude\.ai\/api\/.*\/chat_conversations\/[^/]+\/completion$/,
    /^https:\/\/claude\.ai\/api\/organizations\/[^/]+\/chat_conversations\/[^/]+\/completion$/,
  ],
  gemini: [
    /^https:\/\/gemini\.google\.com\/_\/BardChatUi\/data\/assistant\.lamda\.BardFrontendService\/StreamGenerate$/,
    /^https:\/\/gemini\.google\.com\/u\/\d+\/_\/BardChatUi\/data\//,
  ],
  copilot: [
    /^https:\/\/copilot\.microsoft\.com\/c\/api\/conversations/,
    /^https:\/\/sydney\.bing\.com\/sydney\/ChatHub/,
  ],
  perplexity: [
    /^https:\/\/www\.perplexity\.ai\/socket\.io\//,
    /^https:\/\/api\.perplexity\.ai\/chat\/completions/,
  ],
  poe: [
    /^https:\/\/poe\.com\/api\/gql_POST$/,
  ],
} as const;

/**
 * Check if a URL matches any LLM API endpoint.
 */
export function matchesLLMEndpoint(url: string): { matches: boolean; service: string | null } {
  for (const [service, patterns] of Object.entries(API_PATTERNS)) {
    for (const pattern of patterns) {
      if (pattern.test(url)) {
        return { matches: true, service };
      }
    }
  }
  return { matches: false, service: null };
}

/**
 * Extract prompt from different API request formats.
 */
export function extractPromptFromBody(body: string | object, service: string): string | null {
  try {
    const data = typeof body === 'string' ? JSON.parse(body) : body;

    switch (service) {
      case 'chatgpt':
        // ChatGPT uses { messages: [{ content: { parts: ["prompt"] } }] }
        // or { messages: [{ content: "prompt" }] }
        if (data.messages && Array.isArray(data.messages)) {
          const lastMessage = data.messages[data.messages.length - 1];
          if (lastMessage?.content?.parts) {
            return lastMessage.content.parts.join('\n');
          }
          if (typeof lastMessage?.content === 'string') {
            return lastMessage.content;
          }
        }
        // Also check for prompt field directly
        if (data.prompt) {
          return data.prompt;
        }
        break;

      case 'claude':
        // Claude uses { prompt: "text" } or { message: "text" }
        if (data.prompt) return data.prompt;
        if (data.message) return data.message;
        if (data.content) return data.content;
        break;

      case 'gemini':
        // Gemini uses protobuf-like format, often as array
        // Try to find text in nested arrays
        if (Array.isArray(data)) {
          const text = findTextInNestedArray(data);
          if (text) return text;
        }
        if (data.prompt) return data.prompt;
        if (data.text) return data.text;
        break;

      case 'copilot':
        // Copilot/Bing uses { message: "text" } or { prompt: "text" }
        if (data.message) return data.message;
        if (data.prompt) return data.prompt;
        if (data.text) return data.text;
        break;

      case 'perplexity':
        // Perplexity uses standard OpenAI-like format
        if (data.messages && Array.isArray(data.messages)) {
          const lastMessage = data.messages[data.messages.length - 1];
          if (lastMessage?.content) {
            return typeof lastMessage.content === 'string'
              ? lastMessage.content
              : JSON.stringify(lastMessage.content);
          }
        }
        if (data.query) return data.query;
        break;

      case 'poe':
        // Poe GraphQL format
        if (data.variables?.message) return data.variables.message;
        if (data.variables?.query) return data.variables.query;
        break;
    }

    // Generic fallback: look for common field names
    for (const field of ['prompt', 'message', 'content', 'text', 'query', 'input']) {
      if (data[field] && typeof data[field] === 'string') {
        return data[field];
      }
    }

    return null;
  } catch {
    // If body isn't JSON, check if it's a string prompt
    if (typeof body === 'string' && body.length > 0 && body.length < 10000) {
      // Avoid returning huge blobs
      return body;
    }
    return null;
  }
}

/**
 * Recursively find text in nested arrays (for Gemini's format).
 */
function findTextInNestedArray(arr: unknown[], depth = 0): string | null {
  if (depth > 10) return null; // Prevent infinite recursion

  for (const item of arr) {
    if (typeof item === 'string' && item.length > 10 && item.length < 10000) {
      // Likely a prompt if it's a reasonably sized string
      return item;
    }
    if (Array.isArray(item)) {
      const found = findTextInNestedArray(item, depth + 1);
      if (found) return found;
    }
  }
  return null;
}

/**
 * Callback type for interception results.
 */
export type InterceptCallback = (result: {
  allowed: boolean;
  prompt: string;
  service: string;
  response?: CheckResponse;
  error?: string;
}) => void;

/**
 * State for the interceptor.
 */
let isInterceptorInstalled = false;
let interceptCallback: InterceptCallback | null = null;
let failMode: 'open' | 'closed' = 'closed';

/**
 * Set the fail mode for when Aegis service is unavailable.
 */
export function setFailMode(mode: 'open' | 'closed'): void {
  failMode = mode;
}

/**
 * Set callback for interception events (for UI feedback).
 */
export function setInterceptCallback(callback: InterceptCallback | null): void {
  interceptCallback = callback;
}

/**
 * Check a prompt with the Aegis service.
 * Returns true if the request should be allowed.
 */
async function checkWithAegis(prompt: string, service: string): Promise<{ allowed: boolean; response?: CheckResponse; error?: string }> {
  try {
    const response = await checkPrompt(prompt);

    if (shouldBlock(response.action)) {
      interceptCallback?.({ allowed: false, prompt, service, response });
      return { allowed: false, response };
    }

    if (shouldWarn(response.action)) {
      // For network interception, we can't easily show a "proceed anyway" dialog
      // So we treat warnings as blocks for now
      interceptCallback?.({ allowed: false, prompt, service, response });
      return { allowed: false, response };
    }

    interceptCallback?.({ allowed: true, prompt, service, response });
    return { allowed: true, response };
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : 'Unknown error';

    // Fail mode determines behavior when service is unavailable
    if (failMode === 'closed') {
      interceptCallback?.({ allowed: false, prompt, service, error: errorMsg });
      return { allowed: false, error: errorMsg };
    } else {
      interceptCallback?.({ allowed: true, prompt, service, error: errorMsg });
      return { allowed: true, error: errorMsg };
    }
  }
}

/**
 * Install the network interceptor.
 * This injects a script that overrides fetch() and XMLHttpRequest.
 */
export function installNetworkInterceptor(): void {
  if (isInterceptorInstalled) return;
  isInterceptorInstalled = true;

  // Create the interceptor script to inject into the page context
  const script = document.createElement('script');
  script.textContent = `
(function() {
  'use strict';

  const API_PATTERNS = ${JSON.stringify(Object.fromEntries(
    Object.entries(API_PATTERNS).map(([k, v]) => [k, v.map(r => r.source)])
  ))};

  function matchesLLMEndpoint(url) {
    for (const [service, patterns] of Object.entries(API_PATTERNS)) {
      for (const pattern of patterns) {
        if (new RegExp(pattern).test(url)) {
          return { matches: true, service };
        }
      }
    }
    return { matches: false, service: null };
  }

  // Store original functions
  const originalFetch = window.fetch;
  const originalXHROpen = XMLHttpRequest.prototype.open;
  const originalXHRSend = XMLHttpRequest.prototype.send;

  // Override fetch
  window.fetch = async function(input, init) {
    const url = typeof input === 'string' ? input : input.url;
    const match = matchesLLMEndpoint(url);

    if (match.matches && init?.body) {
      // Dispatch event for content script to handle
      const event = new CustomEvent('aegis-intercept-request', {
        detail: {
          url: url,
          body: init.body,
          service: match.service,
          requestId: Math.random().toString(36).substring(7)
        }
      });

      // Wait for approval from content script
      const approved = await new Promise((resolve) => {
        const timeout = setTimeout(() => {
          // Timeout after 10 seconds - use fail mode
          resolve(window.__aegisFailMode !== 'closed');
        }, 10000);

        const handler = (e) => {
          if (e.detail.requestId === event.detail.requestId) {
            clearTimeout(timeout);
            window.removeEventListener('aegis-intercept-response', handler);
            resolve(e.detail.allowed);
          }
        };
        window.addEventListener('aegis-intercept-response', handler);
        window.dispatchEvent(event);
      });

      if (!approved) {
        // Return a blocked response
        return new Response(JSON.stringify({
          error: 'Request blocked by Aegis safety filter'
        }), {
          status: 403,
          statusText: 'Blocked by Aegis',
          headers: { 'Content-Type': 'application/json' }
        });
      }
    }

    return originalFetch.apply(this, arguments);
  };

  // Override XMLHttpRequest
  XMLHttpRequest.prototype.open = function(method, url) {
    this._aegisUrl = url;
    this._aegisMatch = matchesLLMEndpoint(url);
    return originalXHROpen.apply(this, arguments);
  };

  XMLHttpRequest.prototype.send = function(body) {
    if (this._aegisMatch?.matches && body) {
      const xhr = this;
      const event = new CustomEvent('aegis-intercept-request', {
        detail: {
          url: this._aegisUrl,
          body: body,
          service: this._aegisMatch.service,
          requestId: Math.random().toString(36).substring(7),
          isXHR: true
        }
      });

      // For XHR, we need to handle this synchronously or abort
      // Since we can't easily make this async, dispatch and proceed
      // The content script will block future requests if needed
      window.dispatchEvent(event);
    }

    return originalXHRSend.apply(this, arguments);
  };

  // Mark as installed
  window.__aegisInterceptorInstalled = true;
})();
`;

  // Inject into page context
  (document.head || document.documentElement).appendChild(script);
  script.remove();

  // Listen for intercepted requests from page context
  window.addEventListener('aegis-intercept-request', async (e: Event) => {
    const customEvent = e as CustomEvent;
    const { url, body, service, requestId } = customEvent.detail;

    // Extract prompt from body
    const prompt = extractPromptFromBody(body, service);

    if (!prompt) {
      // Can't extract prompt, allow the request
      window.dispatchEvent(new CustomEvent('aegis-intercept-response', {
        detail: { requestId, allowed: true }
      }));
      return;
    }

    // Check with Aegis
    const result = await checkWithAegis(prompt, service);

    // Send response back to page context
    window.dispatchEvent(new CustomEvent('aegis-intercept-response', {
      detail: { requestId, allowed: result.allowed }
    }));
  });

  console.log('[Aegis] Network interceptor installed');
}

/**
 * Uninstall the network interceptor (for cleanup).
 */
export function uninstallNetworkInterceptor(): void {
  // We can't easily uninstall the injected script, but we can stop responding
  isInterceptorInstalled = false;
  interceptCallback = null;
}
