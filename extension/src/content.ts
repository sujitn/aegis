/**
 * Aegis content script - intercepts prompts on AI chat sites.
 *
 * Uses a hybrid approach for maximum reliability:
 * 1. Primary: Network request interception (immune to DOM changes)
 * 2. Fallback: DOM-based interception for edge cases
 *
 * Supports fail-closed mode for maximum safety.
 */

import { checkPrompt, shouldBlock, shouldWarn, type CheckResponse } from './api.js';
import { getSiteHandler, type SiteHandler } from './sites/index.js';
import {
  installNetworkInterceptor,
  setFailMode,
  setInterceptCallback,
  setResponseInterceptCallback,
  type InterceptCallback,
  type ResponseInterceptCallback
} from './interceptor.js';

const OVERLAY_ID = 'aegis-overlay';

// Aegis lock-open logo SVG (matches app icon)
const AEGIS_LOGO_SVG = `
<svg class="aegis-logo" viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg">
  <path d="M40,18H16V13a7,7,0,0,1,7-7h2a7.1,7.1,0,0,1,5,2.1,2,2,0,0,0,2.2.5h.1a1.9,1.9,0,0,0,.6-3.1A10.9,10.9,0,0,0,25,2H23A11,11,0,0,0,12,13v5H8a2,2,0,0,0-2,2V44a2,2,0,0,0,2,2H40a2,2,0,0,0,2-2V20A2,2,0,0,0,40,18ZM38,42H10V22H38Z"/>
  <path d="M15,40a2,2,0,0,1-1.3-3.5L19,32l-5.3-4.5a2,2,0,0,1,2.6-3l7,6a2,2,0,0,1,0,3l-7,6A1.9,1.9,0,0,1,15,40Z" opacity="0.7"/>
  <path d="M33,38H27a2,2,0,0,1,0-4h6a2,2,0,0,1,0,4Z" opacity="0.7"/>
</svg>`;

// Aegis branding header HTML
const AEGIS_BRAND_HEADER = `
<div class="aegis-brand">
  ${AEGIS_LOGO_SVG}
  <span class="aegis-brand-text">Aegis</span>
</div>`;

let isChecking = false;
let siteHandler: SiteHandler | null = null;
let failMode: 'open' | 'closed' = 'closed'; // Default to fail-closed for safety

/**
 * Load settings from storage.
 */
async function loadSettings(): Promise<void> {
  try {
    const result = await chrome.storage.local.get(['failMode']);
    if (result.failMode) {
      failMode = result.failMode;
    }
    // Also set fail mode for network interceptor
    setFailMode(failMode);
  } catch {
    // Use default
  }
}

/**
 * Listen for settings changes.
 */
chrome.storage.onChanged.addListener((changes) => {
  if (changes.failMode) {
    failMode = changes.failMode.newValue || 'closed';
    setFailMode(failMode);
  }
});

/**
 * Create and show the checking overlay.
 */
function showCheckingOverlay(): void {
  removeOverlay();

  const overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'aegis-overlay aegis-checking';
  overlay.innerHTML = `
    <div class="aegis-overlay-content">
      ${AEGIS_BRAND_HEADER}
      <div class="aegis-spinner"></div>
      <div class="aegis-text">Checking prompt safety...</div>
    </div>
  `;

  const container = siteHandler?.getOverlayContainer() || document.body;
  container.appendChild(overlay);
}

/**
 * Show the blocked message overlay.
 */
function showBlockedOverlay(response: CheckResponse): void {
  removeOverlay();

  const categories = response.categories
    .map(c => `${c.category} (${Math.round(c.confidence * 100)}%)`)
    .join(', ');

  const overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'aegis-overlay aegis-blocked';
  overlay.innerHTML = `
    <div class="aegis-overlay-content">
      ${AEGIS_BRAND_HEADER}
      <div class="aegis-icon">&#128683;</div>
      <div class="aegis-title">Prompt Blocked</div>
      <div class="aegis-text">${response.reason}</div>
      ${categories ? `<div class="aegis-categories">Detected: ${categories}</div>` : ''}
      <button class="aegis-dismiss">Dismiss</button>
    </div>
  `;

  const dismissBtn = overlay.querySelector('.aegis-dismiss');
  dismissBtn?.addEventListener('click', removeOverlay);

  const container = siteHandler?.getOverlayContainer() || document.body;
  container.appendChild(overlay);

  // Auto-dismiss after 5 seconds
  setTimeout(removeOverlay, 5000);
}

/**
 * Show warning overlay with option to proceed.
 */
function showWarningOverlay(response: CheckResponse, onProceed: () => void): void {
  removeOverlay();

  const categories = response.categories
    .map(c => `${c.category} (${Math.round(c.confidence * 100)}%)`)
    .join(', ');

  const overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'aegis-overlay aegis-warning';
  overlay.innerHTML = `
    <div class="aegis-overlay-content">
      ${AEGIS_BRAND_HEADER}
      <div class="aegis-icon">&#9888;</div>
      <div class="aegis-title">Warning</div>
      <div class="aegis-text">${response.reason}</div>
      ${categories ? `<div class="aegis-categories">Detected: ${categories}</div>` : ''}
      <div class="aegis-buttons">
        <button class="aegis-cancel">Cancel</button>
        <button class="aegis-proceed">Send Anyway</button>
      </div>
    </div>
  `;

  const cancelBtn = overlay.querySelector('.aegis-cancel');
  const proceedBtn = overlay.querySelector('.aegis-proceed');

  cancelBtn?.addEventListener('click', removeOverlay);
  proceedBtn?.addEventListener('click', () => {
    removeOverlay();
    onProceed();
  });

  const container = siteHandler?.getOverlayContainer() || document.body;
  container.appendChild(overlay);
}

/**
 * Show service unavailable message (fail-open mode).
 */
function showServiceUnavailableAllowed(): void {
  removeOverlay();

  const overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'aegis-overlay aegis-offline';
  overlay.innerHTML = `
    <div class="aegis-overlay-content">
      ${AEGIS_BRAND_HEADER}
      <div class="aegis-icon">&#128268;</div>
      <div class="aegis-title">Aegis Unavailable</div>
      <div class="aegis-text">Safety service is not running. Prompt was allowed.</div>
      <button class="aegis-dismiss">OK</button>
    </div>
  `;

  const dismissBtn = overlay.querySelector('.aegis-dismiss');
  dismissBtn?.addEventListener('click', removeOverlay);

  const container = siteHandler?.getOverlayContainer() || document.body;
  container.appendChild(overlay);

  setTimeout(removeOverlay, 3000);
}

/**
 * Show service unavailable blocked message (fail-closed mode).
 */
function showServiceUnavailableBlocked(): void {
  removeOverlay();

  const overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'aegis-overlay aegis-blocked';
  overlay.innerHTML = `
    <div class="aegis-overlay-content">
      ${AEGIS_BRAND_HEADER}
      <div class="aegis-icon">&#128274;</div>
      <div class="aegis-title">Prompt Blocked</div>
      <div class="aegis-text">Safety service is not running. Prompts are blocked for your protection.</div>
      <div class="aegis-hint">Please start the Aegis app to send messages.</div>
      <button class="aegis-dismiss">Dismiss</button>
    </div>
  `;

  const dismissBtn = overlay.querySelector('.aegis-dismiss');
  dismissBtn?.addEventListener('click', removeOverlay);

  const container = siteHandler?.getOverlayContainer() || document.body;
  container.appendChild(overlay);

  // Don't auto-dismiss - user needs to acknowledge
}

/**
 * Show response blocked overlay (streaming response was blocked).
 */
function showResponseBlockedOverlay(response: CheckResponse): void {
  removeOverlay();

  const categories = response.categories
    .map(c => `${c.category} (${Math.round(c.confidence * 100)}%)`)
    .join(', ');

  const overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'aegis-overlay aegis-blocked aegis-response-blocked';
  overlay.innerHTML = `
    <div class="aegis-overlay-content">
      ${AEGIS_BRAND_HEADER}
      <div class="aegis-icon">&#128683;</div>
      <div class="aegis-title">Response Blocked</div>
      <div class="aegis-text">${response.reason}</div>
      ${categories ? `<div class="aegis-categories">Detected: ${categories}</div>` : ''}
      <div class="aegis-hint">The AI response contained potentially unsafe content and was stopped.</div>
      <button class="aegis-dismiss">Dismiss</button>
    </div>
  `;

  const dismissBtn = overlay.querySelector('.aegis-dismiss');
  dismissBtn?.addEventListener('click', removeOverlay);

  const container = siteHandler?.getOverlayContainer() || document.body;
  container.appendChild(overlay);

  // Auto-dismiss after 8 seconds (longer than request block since response is more alarming)
  setTimeout(removeOverlay, 8000);
}

/**
 * Remove the overlay.
 */
function removeOverlay(): void {
  const overlay = document.getElementById(OVERLAY_ID);
  overlay?.remove();
}

/**
 * Intercept and check a prompt before submission.
 * Returns true if the prompt should be blocked.
 */
async function interceptPrompt(prompt: string): Promise<{ blocked: boolean; response?: CheckResponse; serviceError?: boolean }> {
  if (!prompt.trim()) {
    return { blocked: false };
  }

  if (isChecking) {
    return { blocked: true }; // Block while checking
  }

  isChecking = true;
  showCheckingOverlay();

  try {
    const response = await checkPrompt(prompt);
    removeOverlay();

    if (shouldBlock(response.action)) {
      return { blocked: true, response };
    }

    if (shouldWarn(response.action)) {
      return { blocked: true, response }; // Initially block, show warning
    }

    return { blocked: false, response };
  } catch (error) {
    removeOverlay();

    // Fail-closed: block when service unavailable
    if (failMode === 'closed') {
      return { blocked: true, serviceError: true };
    }

    // Fail-open: allow when service unavailable
    return { blocked: false, serviceError: true };
  } finally {
    isChecking = false;
  }
}

/**
 * Handle the result of prompt interception.
 */
function handleInterceptionResult(
  result: { blocked: boolean; response?: CheckResponse; serviceError?: boolean },
  input: HTMLElement
): void {
  if (!result.blocked) {
    if (result.serviceError) {
      showServiceUnavailableAllowed();
    }
    submitOriginal(input);
  } else if (result.serviceError) {
    // Blocked due to service unavailable (fail-closed mode)
    showServiceUnavailableBlocked();
    siteHandler?.clearInput(input);
  } else if (result.response && shouldWarn(result.response.action)) {
    showWarningOverlay(result.response, () => submitOriginal(input));
  } else if (result.response) {
    showBlockedOverlay(result.response);
    siteHandler?.clearInput(input);
  }
}

/**
 * Handle form submission.
 */
function handleSubmit(event: Event, input: HTMLElement): void {
  const prompt = siteHandler?.getPromptText(input) || '';

  if (!prompt.trim()) {
    return; // Allow empty submissions
  }

  // Prevent default submission
  event.preventDefault();
  event.stopPropagation();
  event.stopImmediatePropagation();

  // Check the prompt
  interceptPrompt(prompt).then((result) => handleInterceptionResult(result, input));
}

/**
 * Submit the form programmatically (bypass our interception).
 */
function submitOriginal(input: HTMLElement): void {
  // Find and click the submit button
  const buttons = siteHandler?.findSubmitButtons() || [];
  for (const button of buttons) {
    if (button instanceof HTMLButtonElement && !button.disabled) {
      // Temporarily mark as allowed
      button.dataset.aegisAllowed = 'true';
      button.click();
      delete button.dataset.aegisAllowed;
      return;
    }
  }

  // Fallback: dispatch Enter keydown
  input.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'Enter',
    code: 'Enter',
    keyCode: 13,
    which: 13,
    bubbles: true,
    cancelable: true,
  }));
}

/**
 * Handle keyboard events on input.
 */
function handleKeydown(event: KeyboardEvent, input: HTMLElement): void {
  // Check for Enter without Shift (submit)
  if (event.key === 'Enter' && !event.shiftKey) {
    const prompt = siteHandler?.getPromptText(input) || '';

    if (!prompt.trim()) {
      return; // Allow empty submissions
    }

    // Prevent default
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();

    // Check the prompt
    interceptPrompt(prompt).then((result) => handleInterceptionResult(result, input));
  }
}

/**
 * Set up interception on an input element.
 */
function setupInputInterception(input: HTMLElement): void {
  if (input.dataset.aegisIntercepted) {
    return; // Already set up
  }
  input.dataset.aegisIntercepted = 'true';

  // Intercept Enter key
  input.addEventListener('keydown', (e) => handleKeydown(e as KeyboardEvent, input), true);
}

/**
 * Set up interception on submit buttons.
 */
function setupButtonInterception(button: HTMLElement, inputs: HTMLElement[]): void {
  if (button.dataset.aegisIntercepted) {
    return; // Already set up
  }
  button.dataset.aegisIntercepted = 'true';

  button.addEventListener('click', (e) => {
    // Skip if we're allowing this submission
    if ((button as HTMLButtonElement).dataset.aegisAllowed) {
      return;
    }

    // Find the relevant input
    const input = inputs[0];
    if (input) {
      handleSubmit(e, input);
    }
  }, true);
}

/**
 * Handle network interception callback for UI feedback.
 */
function handleNetworkInterception(result: {
  allowed: boolean;
  prompt: string;
  service: string;
  response?: CheckResponse;
  error?: string;
}): void {
  if (!result.allowed) {
    if (result.error) {
      // Service unavailable in fail-closed mode
      showServiceUnavailableBlocked();
    } else if (result.response) {
      showBlockedOverlay(result.response);
    }
  }
}

/**
 * Handle streaming response interception callback for UI feedback.
 */
function handleResponseInterception(result: {
  blocked: boolean;
  content: string;
  service: string;
  response?: CheckResponse;
  error?: string;
}): void {
  if (result.blocked && result.response) {
    showResponseBlockedOverlay(result.response);
    console.log(`[Aegis] Streaming response blocked on ${result.service}: ${result.response.reason}`);
  } else if (result.error) {
    // Log error but don't block (fail-open for responses is safer)
    console.warn(`[Aegis] Response check error on ${result.service}: ${result.error}`);
  }
}

/**
 * Initialize interception for the current page.
 */
async function initialize(): Promise<void> {
  // Load settings first
  await loadSettings();

  siteHandler = getSiteHandler();

  if (!siteHandler) {
    return;
  }

  // Primary: Install network request interceptor (immune to DOM changes)
  // Also handles streaming response interception
  setInterceptCallback(handleNetworkInterception);
  setResponseInterceptCallback(handleResponseInterception);
  installNetworkInterceptor();

  // Fallback: Set up DOM-based interception for edge cases
  // This catches submissions that might bypass network interception
  const inputs = siteHandler.findInputs();
  const buttons = siteHandler.findSubmitButtons();

  inputs.forEach(setupInputInterception);
  buttons.forEach(btn => setupButtonInterception(btn, inputs));

  // Watch for dynamically added elements (for DOM fallback)
  const observer = new MutationObserver(() => {
    if (!siteHandler) return;

    const newInputs = siteHandler.findInputs();
    const newButtons = siteHandler.findSubmitButtons();

    newInputs.forEach(setupInputInterception);
    newButtons.forEach(btn => setupButtonInterception(btn, newInputs));
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
  });

  // Notify background that we're active
  chrome.runtime.sendMessage({ type: 'CONTENT_LOADED', site: siteHandler.name });
  console.log(`[Aegis] Initialized on ${siteHandler.name} (network + DOM interception)`);
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initialize);
} else {
  initialize();
}
