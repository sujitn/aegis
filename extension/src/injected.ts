/**
 * Injected script that runs in page context to intercept fetch/XHR/WebSocket.
 * This file is compiled separately and loaded via URL to avoid CSP inline script issues.
 */

// Extend Window interface for Aegis properties
interface AegisWindow extends Window {
  __aegisStreamingConfig: {
    bufferSize: number;
    bufferTimeout: number;
    enabled: boolean;
  };
  __aegisFailMode: string;
  __aegisInterceptorInstalled: boolean;
}

const aegisWindow = window as unknown as AegisWindow;

(function () {
  'use strict';

  // Read configuration from DOM element (set by content script to avoid CSP issues)
  const configElement = document.getElementById('__aegis_config__');
  let API_PATTERNS: Record<string, string[]> = {};
  let WS_PATTERNS: Record<string, string[]> = {};

  if (configElement) {
    try {
      const patterns = JSON.parse(configElement.dataset.patterns || '{}');
      API_PATTERNS = patterns.api || {};
      WS_PATTERNS = patterns.ws || {};

      const config = JSON.parse(configElement.dataset.config || '{}');
      aegisWindow.__aegisStreamingConfig = {
        bufferSize: config.bufferSize || 500,
        bufferTimeout: config.bufferTimeout || 2000,
        enabled: config.enabled !== false,
      };

      aegisWindow.__aegisFailMode = configElement.dataset.failMode || 'closed';
    } catch (e) {
      console.error('[Aegis] Failed to parse config:', e);
    }
  }

  // Default config if not set
  if (!aegisWindow.__aegisStreamingConfig) {
    aegisWindow.__aegisStreamingConfig = {
      bufferSize: 500,
      bufferTimeout: 2000,
      enabled: true,
    };
  }

  function matchesLLMEndpoint(url: string): { matches: boolean; service: string | null } {
    for (const [service, patterns] of Object.entries(API_PATTERNS)) {
      for (const pattern of patterns) {
        if (new RegExp(pattern).test(url)) {
          return { matches: true, service };
        }
      }
    }
    return { matches: false, service: null };
  }

  function matchesWebSocketEndpoint(url: string): { matches: boolean; service: string | null } {
    for (const [service, patterns] of Object.entries(WS_PATTERNS)) {
      for (const pattern of patterns) {
        if (new RegExp(pattern).test(url)) {
          return { matches: true, service };
        }
      }
    }
    return { matches: false, service: null };
  }

  /**
   * Extract prompt from WebSocket message based on service.
   */
  function extractPromptFromWSMessage(data: string | ArrayBuffer, service: string): string | null {
    try {
      const text = typeof data === 'string' ? data : new TextDecoder().decode(data);

      if (!text || text.length < 5) return null;

      // Socket.io protocol: starts with a number (packet type)
      if (text.match(/^\d+/)) {
        const jsonStart = text.search(/[\[\{]/);
        if (jsonStart > -1) {
          const json = JSON.parse(text.slice(jsonStart));

          if (Array.isArray(json) && json.length >= 2) {
            const payload = json[1];
            if (payload?.query) return payload.query;
            if (payload?.content) return payload.content;
            if (payload?.text) return payload.text;
            if (payload?.message) return payload.message;
          }
          return null;
        }
      }

      const json = JSON.parse(text);

      switch (service) {
        case 'copilot':
          if (json.arguments?.[0]?.messages) {
            const msgs = json.arguments[0].messages;
            const lastMsg = msgs[msgs.length - 1];
            if (lastMsg?.text) return lastMsg.text;
            if (lastMsg?.content) return lastMsg.content;
          }
          if (json.message) return json.message;
          if (json.text) return json.text;
          break;

        case 'perplexity':
          if (json.query) return json.query;
          if (json.content) return json.content;
          if (json.text) return json.text;
          break;

        case 'poe':
          if (json.message) return json.message;
          if (json.query) return json.query;
          if (json.text) return json.text;
          break;
      }

      if (json.query) return json.query;
      if (json.message) return json.message;
      if (json.text) return json.text;
      if (json.content) return json.content;

      return null;
    } catch {
      return null;
    }
  }

  /**
   * Parse SSE content to extract actual text.
   */
  function parseSSEContent(chunk: string): string {
    const lines = chunk.split('\n');
    const contents: string[] = [];

    for (const line of lines) {
      if (!line || line.startsWith(':')) continue;

      if (line.startsWith('data: ')) {
        const data = line.slice(6);
        if (data === '[DONE]') continue;

        try {
          const json = JSON.parse(data);
          if (json.choices?.[0]?.delta?.content) {
            contents.push(json.choices[0].delta.content);
          } else if (json.delta?.text) {
            contents.push(json.delta.text);
          } else if (json.completion) {
            contents.push(json.completion);
          } else if (json.content) {
            contents.push(typeof json.content === 'string' ? json.content : JSON.stringify(json.content));
          } else if (json.text) {
            contents.push(json.text);
          }
        } catch {
          contents.push(data);
        }
      }
    }

    return contents.join('');
  }

  /**
   * Create a wrapped response with streaming interception.
   */
  function wrapStreamingResponse(response: Response, service: string, requestId: string): Response {
    const contentType = response.headers.get('content-type');
    const isStreaming =
      contentType?.includes('text/event-stream') ||
      contentType?.includes('application/x-ndjson') ||
      contentType?.includes('application/octet-stream') ||
      response.headers.get('transfer-encoding') === 'chunked';

    if (!isStreaming || !aegisWindow.__aegisStreamingConfig?.enabled || !response.body) {
      return response;
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    let checkedInitial = false;
    let isBlocked = false;
    const abortController = new AbortController();

    const wrappedStream = new ReadableStream({
      async start(controller) {
        const config = aegisWindow.__aegisStreamingConfig;
        let timeoutId: ReturnType<typeof setTimeout> | null = null;

        if (config.bufferTimeout > 0) {
          timeoutId = setTimeout(async () => {
            if (!checkedInitial && buffer.length > 0) {
              checkedInitial = true;
              await checkBuffer();
            }
          }, config.bufferTimeout);
        }

        async function checkBuffer() {
          const parsedContent = parseSSEContent(buffer);

          if (parsedContent.length > 0) {
            const checkEvent = new CustomEvent('aegis-check-response', {
              detail: {
                content: parsedContent,
                service: service,
                requestId: requestId,
              },
            });

            const blocked = await new Promise<boolean>((resolve) => {
              const checkTimeout = setTimeout(() => resolve(false), 5000);

              const handler = (e: Event) => {
                const ce = e as CustomEvent;
                if (ce.detail.requestId === requestId) {
                  clearTimeout(checkTimeout);
                  window.removeEventListener('aegis-response-check-result', handler);
                  resolve(ce.detail.blocked);
                }
              };
              window.addEventListener('aegis-response-check-result', handler);
              window.dispatchEvent(checkEvent);
            });

            if (blocked) {
              isBlocked = true;
              abortController.abort();
            }
          }
        }

        try {
          while (true) {
            const { done, value } = await reader.read();

            if (done) {
              if (timeoutId) clearTimeout(timeoutId);

              if (!checkedInitial && buffer.length > 0) {
                checkedInitial = true;
                await checkBuffer();
              }

              if (!isBlocked) {
                controller.close();
              }
              break;
            }

            const chunk = decoder.decode(value, { stream: true });
            buffer += chunk;

            if (!checkedInitial && buffer.length >= config.bufferSize) {
              checkedInitial = true;
              if (timeoutId) clearTimeout(timeoutId);
              await checkBuffer();
            }

            if (isBlocked) {
              const blockedMessage = new TextEncoder().encode(
                'data: {"error": "Response blocked by Aegis safety filter"}\n\n'
              );
              controller.enqueue(blockedMessage);
              controller.close();
              break;
            }

            controller.enqueue(value);
          }
        } catch (error) {
          if (timeoutId) clearTimeout(timeoutId);
          if (!isBlocked) {
            controller.error(error);
          }
        }
      },

      cancel() {
        reader.cancel();
        abortController.abort();
      },
    });

    return new Response(wrappedStream, {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  }

  // Store original functions
  const originalFetch = window.fetch;
  const originalXHROpen = XMLHttpRequest.prototype.open;
  const originalXHRSend = XMLHttpRequest.prototype.send;

  // Override fetch
  window.fetch = async function (input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    const match = matchesLLMEndpoint(url);

    if (match.matches && init?.body) {
      const requestId = Math.random().toString(36).substring(7);

      const event = new CustomEvent('aegis-intercept-request', {
        detail: {
          url: url,
          body: init.body,
          service: match.service,
          requestId: requestId,
        },
      });

      const approved = await new Promise<boolean>((resolve) => {
        const timeout = setTimeout(() => {
          resolve(aegisWindow.__aegisFailMode !== 'closed');
        }, 10000);

        const handler = (e: Event) => {
          const ce = e as CustomEvent;
          if (ce.detail.requestId === requestId) {
            clearTimeout(timeout);
            window.removeEventListener('aegis-intercept-response', handler);
            resolve(ce.detail.allowed);
          }
        };
        window.addEventListener('aegis-intercept-response', handler);
        window.dispatchEvent(event);
      });

      if (!approved) {
        return new Response(
          JSON.stringify({
            error: 'Request blocked by Aegis safety filter',
          }),
          {
            status: 403,
            statusText: 'Blocked by Aegis',
            headers: { 'Content-Type': 'application/json' },
          }
        );
      }

      const response = await originalFetch.apply(window, [input, init]);
      return wrapStreamingResponse(response, match.service!, requestId);
    }

    return originalFetch.apply(window, [input, init]);
  };

  // Extend XMLHttpRequest type
  interface AegisXHR extends XMLHttpRequest {
    _aegisUrl?: string;
    _aegisMatch?: { matches: boolean; service: string | null };
  }

  // Override XMLHttpRequest
  XMLHttpRequest.prototype.open = function (
    this: AegisXHR,
    method: string,
    url: string | URL,
    async?: boolean,
    username?: string | null,
    password?: string | null
  ) {
    this._aegisUrl = url.toString();
    this._aegisMatch = matchesLLMEndpoint(url.toString());
    return originalXHROpen.call(this, method, url, async ?? true, username, password);
  };

  XMLHttpRequest.prototype.send = function (this: AegisXHR, body?: Document | XMLHttpRequestBodyInit | null) {
    if (this._aegisMatch?.matches && body) {
      const xhr = this;
      const requestId = Math.random().toString(36).substring(7);
      const event = new CustomEvent('aegis-intercept-request', {
        detail: {
          url: this._aegisUrl,
          body: body,
          service: this._aegisMatch.service,
          requestId: requestId,
          isXHR: true,
        },
      });

      window.dispatchEvent(event);

      if (aegisWindow.__aegisStreamingConfig?.enabled) {
        const originalOnReadyStateChange = xhr.onreadystatechange;
        let responseBuffer = '';
        let checkedResponse = false;

        xhr.onreadystatechange = function (ev: Event) {
          if (xhr.readyState === 3 && !checkedResponse) {
            responseBuffer = xhr.responseText;

            if (responseBuffer.length >= aegisWindow.__aegisStreamingConfig.bufferSize) {
              checkedResponse = true;
              const parsedContent = parseSSEContent(responseBuffer);

              if (parsedContent.length > 0) {
                window.dispatchEvent(
                  new CustomEvent('aegis-check-response', {
                    detail: {
                      content: parsedContent,
                      service: xhr._aegisMatch?.service,
                      requestId: requestId,
                    },
                  })
                );
              }
            }
          }

          if (originalOnReadyStateChange) {
            originalOnReadyStateChange.call(this, ev);
          }
        };
      }
    }

    return originalXHRSend.call(this, body);
  };

  // Override WebSocket
  const OriginalWebSocket = window.WebSocket;

  function AegisWebSocket(url: string | URL, protocols?: string | string[]): WebSocket {
    const urlStr = url.toString();
    const match = matchesWebSocketEndpoint(urlStr);
    const ws = protocols ? new OriginalWebSocket(url, protocols) : new OriginalWebSocket(url);

    if (match.matches) {
      const service = match.service!;
      const originalSend = ws.send.bind(ws);

      ws.send = function (data: string | ArrayBufferLike | Blob | ArrayBufferView) {
        if (typeof data === 'string' || data instanceof ArrayBuffer) {
          const prompt = extractPromptFromWSMessage(data, service);

          if (prompt && prompt.length > 10) {
            const requestId = Math.random().toString(36).substring(7);

            window.dispatchEvent(
              new CustomEvent('aegis-intercept-request', {
                detail: {
                  url: urlStr,
                  body: data,
                  service: service,
                  requestId: requestId,
                  isWebSocket: true,
                  extractedPrompt: prompt,
                },
              })
            );
          }
        }

        return originalSend(data);
      };

      const originalAddEventListener = ws.addEventListener.bind(ws);
      ws.addEventListener = function <K extends keyof WebSocketEventMap>(
        type: K,
        listener: (this: WebSocket, ev: WebSocketEventMap[K]) => void,
        options?: boolean | AddEventListenerOptions
      ) {
        if (type === 'message' && aegisWindow.__aegisStreamingConfig?.enabled) {
          const wrappedListener = function (this: WebSocket, event: MessageEvent) {
            try {
              const text = typeof event.data === 'string' ? event.data : new TextDecoder().decode(event.data);

              if (text && text.length > 50) {
                const parsedContent = parseSSEContent(text);
                if (parsedContent && parsedContent.length > 20) {
                  window.dispatchEvent(
                    new CustomEvent('aegis-check-response', {
                      detail: {
                        content: parsedContent,
                        service: service,
                        requestId: Math.random().toString(36).substring(7),
                      },
                    })
                  );
                }
              }
            } catch {
              // Ignore
            }

            (listener as (this: WebSocket, ev: MessageEvent) => void).call(this, event);
          };

          return originalAddEventListener(type, wrappedListener as EventListener, options);
        }

        return originalAddEventListener(type, listener as EventListener, options);
      };

      let _onmessage: ((this: WebSocket, ev: MessageEvent) => void) | null = null;
      Object.defineProperty(ws, 'onmessage', {
        get() {
          return _onmessage;
        },
        set(handler: ((this: WebSocket, ev: MessageEvent) => void) | null) {
          if (handler && aegisWindow.__aegisStreamingConfig?.enabled) {
            _onmessage = function (this: WebSocket, event: MessageEvent) {
              try {
                const text = typeof event.data === 'string' ? event.data : new TextDecoder().decode(event.data);

                if (text && text.length > 50) {
                  const parsedContent = parseSSEContent(text);
                  if (parsedContent && parsedContent.length > 20) {
                    window.dispatchEvent(
                      new CustomEvent('aegis-check-response', {
                        detail: {
                          content: parsedContent,
                          service: service,
                          requestId: Math.random().toString(36).substring(7),
                        },
                      })
                    );
                  }
                }
              } catch {
                // Ignore
              }

              handler.call(ws, event);
            };
          } else {
            _onmessage = handler;
          }
        },
      });

      console.log('[Aegis] WebSocket interceptor attached for ' + service);
    }

    return ws;
  }

  // Copy static properties
  AegisWebSocket.CONNECTING = OriginalWebSocket.CONNECTING;
  AegisWebSocket.OPEN = OriginalWebSocket.OPEN;
  AegisWebSocket.CLOSING = OriginalWebSocket.CLOSING;
  AegisWebSocket.CLOSED = OriginalWebSocket.CLOSED;
  AegisWebSocket.prototype = OriginalWebSocket.prototype;

  // Replace WebSocket
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (window as any).WebSocket = AegisWebSocket;

  // Mark as installed
  aegisWindow.__aegisInterceptorInstalled = true;

  // Listen for config updates
  window.addEventListener('aegis-update-streaming-config', (e: Event) => {
    const ce = e as CustomEvent;
    aegisWindow.__aegisStreamingConfig = { ...aegisWindow.__aegisStreamingConfig, ...ce.detail };
  });

  console.log('[Aegis] Network interceptor installed');
})();
