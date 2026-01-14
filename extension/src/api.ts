/**
 * Aegis API client for communicating with the local server.
 */

const API_BASE = 'http://127.0.0.1:8765';
const API_TIMEOUT = 5000;

export interface CategoryMatch {
  category: string;
  confidence: number;
  tier: string;
}

export interface CheckResponse {
  action: 'Allow' | 'Warn' | 'Block';
  reason: string;
  categories: CategoryMatch[];
  latency_ms: number;
}

export interface CheckRequest {
  prompt: string;
  os_username?: string;
}

export type ServiceStatus = 'online' | 'offline' | 'error';

export interface StatusResponse {
  status: ServiceStatus;
  error?: string;
}

/**
 * Check a prompt against the Aegis safety filter.
 */
export async function checkPrompt(prompt: string, osUsername?: string): Promise<CheckResponse> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), API_TIMEOUT);

  try {
    const body: CheckRequest = { prompt };
    if (osUsername) {
      body.os_username = osUsername;
    }

    const response = await fetch(`${API_BASE}/api/check`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(`API error: ${response.status}`);
    }

    return await response.json();
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Check if the Aegis service is available.
 */
export async function checkServiceStatus(): Promise<StatusResponse> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 2000);

  try {
    const response = await fetch(`${API_BASE}/api/stats`, {
      method: 'GET',
      signal: controller.signal,
    });

    if (response.ok) {
      return { status: 'online' };
    }
    return { status: 'error', error: `HTTP ${response.status}` };
  } catch (error) {
    if (error instanceof Error) {
      if (error.name === 'AbortError') {
        return { status: 'offline', error: 'Service timeout' };
      }
      return { status: 'offline', error: error.message };
    }
    return { status: 'offline', error: 'Unknown error' };
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Determine if an action should block the prompt.
 */
export function shouldBlock(action: CheckResponse['action']): boolean {
  return action === 'Block';
}

/**
 * Determine if an action should warn about the prompt.
 */
export function shouldWarn(action: CheckResponse['action']): boolean {
  return action === 'Warn';
}
