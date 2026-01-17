/**
 * Network Request Interceptor for Aegis.
 *
 * Intercepts fetch() and XMLHttpRequest to catch API calls to LLM services.
 * This approach is immune to DOM changes since it intercepts the actual API requests.
 *
 * Also intercepts streaming responses to detect unsafe AI-generated content.
 */

import { checkPrompt, shouldBlock, shouldWarn, type CheckResponse } from './api.js';

/**
 * Configuration for response streaming interception.
 */
export interface StreamingConfig {
  /** Number of characters to buffer before checking (default: 500) */
  bufferSize: number;
  /** Maximum time to wait for buffer to fill in ms (default: 2000) */
  bufferTimeout: number;
  /** Whether to check response content (default: true) */
  enabled: boolean;
}

const DEFAULT_STREAMING_CONFIG: StreamingConfig = {
  bufferSize: 500,
  bufferTimeout: 2000,
  enabled: true,
};

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
    /^https:\/\/copilot\.microsoft\.com\/c\/api\/create/,
    /^https:\/\/copilot\.microsoft\.com\/c\/api\/threads/,
    /^https:\/\/sydney\.bing\.com\/sydney\/ChatHub/,
    /^https:\/\/www\.bing\.com\/turing\/conversation\/create/,
  ],
  perplexity: [
    /^https:\/\/www\.perplexity\.ai\/socket\.io\//,
    /^https:\/\/(www\.)?perplexity\.ai\/api\//,
    /^https:\/\/api\.perplexity\.ai\/chat\/completions/,
  ],
  poe: [
    /^https:\/\/poe\.com\/api\/gql_POST$/,
    /^https:\/\/poe\.com\/api\/send_message/,
    /^https:\/\/poe\.com\/_next\/data\//,
  ],
} as const;

/**
 * WebSocket endpoint patterns for LLM services.
 */
export const WS_PATTERNS = {
  copilot: [
    /^wss:\/\/sydney\.bing\.com\/sydney\/ChatHub/,
    /^wss:\/\/copilot\.microsoft\.com/,
  ],
  perplexity: [
    /^wss:\/\/www\.perplexity\.ai\/socket\.io\//,
    /^wss:\/\/perplexity\.ai\/socket\.io\//,
  ],
  poe: [
    /^wss:\/\/poe\.com\//,
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
 * Check if a URL matches any LLM WebSocket endpoint.
 */
export function matchesWebSocketEndpoint(url: string): { matches: boolean; service: string | null } {
  for (const [service, patterns] of Object.entries(WS_PATTERNS)) {
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
        // Copilot/Bing Sydney uses various formats
        // SignalR format: { arguments: [{ messages: [...] }] }
        if (data.arguments?.[0]?.messages) {
          const msgs = data.arguments[0].messages;
          const lastMsg = msgs[msgs.length - 1];
          if (lastMsg?.text) return lastMsg.text;
          if (lastMsg?.content) return lastMsg.content;
        }
        // Direct message format
        if (data.message) return data.message;
        if (data.prompt) return data.prompt;
        if (data.text) return data.text;
        // Conversation create format
        if (data.userMessage) return data.userMessage;
        break;

      case 'perplexity':
        // Perplexity uses socket.io format and REST API
        // Socket.io format: ["perplexity_ask", { query: "..." }] or similar
        if (Array.isArray(data) && data.length >= 2) {
          const payload = data[1];
          if (payload?.query) return payload.query;
          if (payload?.content) return payload.content;
          if (payload?.text) return payload.text;
        }
        // REST API format (OpenAI-like)
        if (data.messages && Array.isArray(data.messages)) {
          const lastMessage = data.messages[data.messages.length - 1];
          if (lastMessage?.content) {
            return typeof lastMessage.content === 'string'
              ? lastMessage.content
              : JSON.stringify(lastMessage.content);
          }
        }
        if (data.query) return data.query;
        if (data.text) return data.text;
        break;

      case 'poe':
        // Poe GraphQL format
        if (data.variables?.message) return data.variables.message;
        if (data.variables?.query) return data.variables.query;
        if (data.variables?.content) return data.variables.content;
        // Poe also uses direct message format sometimes
        if (data.message) return data.message;
        if (data.query) return data.query;
        // GraphQL query with input
        if (data.variables?.input?.message) return data.variables.input.message;
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
 * Callback type for request interception results.
 */
export type InterceptCallback = (result: {
  allowed: boolean;
  prompt: string;
  service: string;
  response?: CheckResponse;
  error?: string;
}) => void;

/**
 * Callback type for response streaming interception results.
 */
export type ResponseInterceptCallback = (result: {
  blocked: boolean;
  content: string;
  service: string;
  response?: CheckResponse;
  error?: string;
}) => void;

/**
 * State for the interceptor.
 */
let isInterceptorInstalled = false;
let interceptCallback: InterceptCallback | null = null;
let responseInterceptCallback: ResponseInterceptCallback | null = null;
let failMode: 'open' | 'closed' = 'closed';
let streamingConfig: StreamingConfig = { ...DEFAULT_STREAMING_CONFIG };

/**
 * Set the fail mode for when Aegis service is unavailable.
 */
export function setFailMode(mode: 'open' | 'closed'): void {
  failMode = mode;
}

/**
 * Set callback for request interception events (for UI feedback).
 */
export function setInterceptCallback(callback: InterceptCallback | null): void {
  interceptCallback = callback;
}

/**
 * Set callback for response streaming interception events (for UI feedback).
 */
export function setResponseInterceptCallback(callback: ResponseInterceptCallback | null): void {
  responseInterceptCallback = callback;
}

/**
 * Configure streaming response interception.
 */
export function setStreamingConfig(config: Partial<StreamingConfig>): void {
  streamingConfig = { ...streamingConfig, ...config };
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
 * Check streaming response content with the Aegis service.
 * Returns true if the content should be blocked.
 */
async function checkResponseWithAegis(content: string, service: string): Promise<{ blocked: boolean; response?: CheckResponse; error?: string }> {
  try {
    const response = await checkPrompt(content);

    if (shouldBlock(response.action)) {
      responseInterceptCallback?.({ blocked: true, content, service, response });
      return { blocked: true, response };
    }

    if (shouldWarn(response.action)) {
      // For streaming responses, treat warnings as blocks
      responseInterceptCallback?.({ blocked: true, content, service, response });
      return { blocked: true, response };
    }

    return { blocked: false, response };
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : 'Unknown error';

    // For response checking, fail-open is safer (content already being displayed)
    // But we still notify via callback
    responseInterceptCallback?.({ blocked: false, content, service, error: errorMsg });
    return { blocked: false, error: errorMsg };
  }
}

/**
 * Parse Server-Sent Events (SSE) format to extract content.
 * SSE format: "data: {json}\n\n" or "data: [content]\n\n"
 * Also handles socket.io, SignalR, and various streaming formats.
 */
export function parseSSEContent(chunk: string): string {
  const lines = chunk.split('\n');
  const contents: string[] = [];

  for (const line of lines) {
    // Skip empty lines and comments
    if (!line || line.startsWith(':')) continue;

    // Parse data lines
    if (line.startsWith('data: ')) {
      const data = line.slice(6);

      // Skip "[DONE]" markers
      if (data === '[DONE]') continue;

      try {
        // Try to parse as JSON
        const json = JSON.parse(data);
        const extracted = extractContentFromJSON(json);
        if (extracted) contents.push(extracted);
      } catch {
        // Not JSON, use raw data if substantial
        if (data.length > 10 && !data.startsWith('{') && !data.startsWith('[')) {
          contents.push(data);
        }
      }
    }
    // Handle socket.io format (starts with packet type number)
    else if (line.match(/^\d+/)) {
      const jsonStart = line.search(/[\[\{]/);
      if (jsonStart > -1) {
        try {
          const json = JSON.parse(line.slice(jsonStart));
          const extracted = extractContentFromJSON(json);
          if (extracted) contents.push(extracted);
        } catch {
          // Ignore parsing errors
        }
      }
    }
    // Handle plain JSON lines
    else if (line.startsWith('{') || line.startsWith('[')) {
      try {
        const json = JSON.parse(line);
        const extracted = extractContentFromJSON(json);
        if (extracted) contents.push(extracted);
      } catch {
        // Ignore parsing errors
      }
    }
  }

  return contents.join('');
}

/**
 * Extract text content from various JSON response formats.
 */
function extractContentFromJSON(json: unknown): string | null {
  if (!json || typeof json !== 'object') return null;

  const obj = json as Record<string, unknown>;

  // OpenAI/ChatGPT streaming format
  if (obj.choices && Array.isArray(obj.choices)) {
    const choice = obj.choices[0] as Record<string, unknown> | undefined;
    if (choice?.delta) {
      const delta = choice.delta as Record<string, unknown>;
      if (delta.content) return String(delta.content);
    }
    if (choice?.message) {
      const message = choice.message as Record<string, unknown>;
      if (message.content) return String(message.content);
    }
  }

  // Claude streaming format
  if (obj.delta) {
    const delta = obj.delta as Record<string, unknown>;
    if (delta.text) return String(delta.text);
  }
  if (obj.completion) return String(obj.completion);

  // Copilot/Sydney format
  if (obj.arguments && Array.isArray(obj.arguments)) {
    const arg = obj.arguments[0] as Record<string, unknown> | undefined;
    if (arg?.messages && Array.isArray(arg.messages)) {
      const lastMsg = arg.messages[arg.messages.length - 1] as Record<string, unknown>;
      if (lastMsg?.text) return String(lastMsg.text);
      if (lastMsg?.adaptiveCards && Array.isArray(lastMsg.adaptiveCards)) {
        const card = lastMsg.adaptiveCards[0] as Record<string, unknown> | undefined;
        if (card?.body && Array.isArray(card.body)) {
          const texts = (card.body as Record<string, unknown>[])
            .filter(b => b.type === 'TextBlock' && b.text)
            .map(b => String(b.text));
          if (texts.length > 0) return texts.join('\n');
        }
      }
    }
  }

  // Perplexity format
  if (obj.answer) return String(obj.answer);
  if (obj.text) return String(obj.text);

  // Poe format
  if (obj.response) return String(obj.response);

  // Socket.io array format: ["event", { data }]
  if (Array.isArray(json) && json.length >= 2) {
    const payload = json[1] as Record<string, unknown> | undefined;
    if (payload) {
      if (payload.text) return String(payload.text);
      if (payload.content) return String(payload.content);
      if (payload.answer) return String(payload.answer);
      if (payload.response) return String(payload.response);
    }
  }

  // Generic fallbacks
  if (obj.content) {
    return typeof obj.content === 'string' ? obj.content : JSON.stringify(obj.content);
  }
  if (obj.text) return String(obj.text);
  if (obj.message && typeof obj.message === 'string') return obj.message;

  return null;
}

/**
 * Parse streaming response content based on content type.
 */
export function parseStreamingContent(chunk: string, contentType: string | null): string {
  // SSE (Server-Sent Events)
  if (contentType?.includes('text/event-stream') || chunk.includes('data: ')) {
    return parseSSEContent(chunk);
  }

  // NDJSON (Newline-delimited JSON)
  if (contentType?.includes('application/x-ndjson') || contentType?.includes('application/jsonl')) {
    const lines = chunk.split('\n').filter(l => l.trim());
    const contents: string[] = [];

    for (const line of lines) {
      try {
        const json = JSON.parse(line);
        if (json.response) contents.push(json.response);
        else if (json.content) contents.push(json.content);
        else if (json.text) contents.push(json.text);
      } catch {
        // Skip invalid JSON lines
      }
    }

    return contents.join('');
  }

  // Try JSON parsing for regular JSON responses
  try {
    const json = JSON.parse(chunk);
    if (json.response) return json.response;
    if (json.content) return typeof json.content === 'string' ? json.content : JSON.stringify(json.content);
    if (json.text) return json.text;
  } catch {
    // Not JSON
  }

  // Return raw content
  return chunk;
}

/**
 * Install the network interceptor.
 * This injects an external script that overrides fetch(), XMLHttpRequest, and WebSocket.
 * Uses external script file to avoid CSP inline script restrictions.
 * Configuration is passed via DOM data attributes to avoid CSP issues.
 */
export function installNetworkInterceptor(): void {
  if (isInterceptorInstalled) return;
  isInterceptorInstalled = true;

  // Pass configuration via DOM element to avoid CSP inline script restrictions
  const configElement = document.createElement('div');
  configElement.id = '__aegis_config__';
  configElement.style.display = 'none';
  configElement.dataset.patterns = JSON.stringify({
    api: Object.fromEntries(
      Object.entries(API_PATTERNS).map(([k, v]) => [k, v.map(r => r.source)])
    ),
    ws: Object.fromEntries(
      Object.entries(WS_PATTERNS).map(([k, v]) => [k, v.map(r => r.source)])
    )
  });
  configElement.dataset.config = JSON.stringify({
    bufferSize: streamingConfig.bufferSize,
    bufferTimeout: streamingConfig.bufferTimeout,
    enabled: streamingConfig.enabled
  });
  configElement.dataset.failMode = failMode;
  (document.head || document.documentElement).appendChild(configElement);

  // Load the external interceptor script
  const script = document.createElement('script');
  script.src = chrome.runtime.getURL('dist/injected.js');
  script.onload = () => {
    script.remove();
    // Clean up config element after script loads
    configElement.remove();
    console.log('[Aegis] Interceptor script loaded');
  };
  script.onerror = (e) => {
    console.error('[Aegis] Failed to load interceptor script:', e);
    configElement.remove();
  };
  (document.head || document.documentElement).appendChild(script);

  // Listen for intercepted requests from page context
  window.addEventListener('aegis-intercept-request', async (e: Event) => {
    const customEvent = e as CustomEvent;
    const { body, service, requestId, isWebSocket, extractedPrompt } = customEvent.detail;

    // For WebSocket, use the already extracted prompt if available
    let prompt: string | null = null;
    if (isWebSocket && extractedPrompt) {
      prompt = extractedPrompt;
    } else {
      // Extract prompt from body
      prompt = extractPromptFromBody(body, service);
    }

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

  // Listen for streaming response content checks from page context
  window.addEventListener('aegis-check-response', async (e: Event) => {
    const customEvent = e as CustomEvent;
    const { content, service, requestId } = customEvent.detail;

    if (!content || content.length === 0) {
      // No content to check, allow
      window.dispatchEvent(new CustomEvent('aegis-response-check-result', {
        detail: { requestId, blocked: false }
      }));
      return;
    }

    // Check response content with Aegis
    const result = await checkResponseWithAegis(content, service);

    // Send result back to page context
    window.dispatchEvent(new CustomEvent('aegis-response-check-result', {
      detail: { requestId, blocked: result.blocked }
    }));
  });

  console.log('[Aegis] Network interceptor installed (with streaming response interception)');
}

/**
 * Update streaming configuration at runtime.
 */
export function updateStreamingConfig(config: Partial<StreamingConfig>): void {
  streamingConfig = { ...streamingConfig, ...config };

  // Dispatch event to update page context config
  window.dispatchEvent(new CustomEvent('aegis-update-streaming-config', {
    detail: config
  }));
}

/**
 * Uninstall the network interceptor (for cleanup).
 */
export function uninstallNetworkInterceptor(): void {
  // We can't easily uninstall the injected script, but we can stop responding
  isInterceptorInstalled = false;
  interceptCallback = null;
}
