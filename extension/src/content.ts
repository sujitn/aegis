/**
 * Aegis content script - intercepts prompts on AI chat sites.
 */

import { checkPrompt, shouldBlock, shouldWarn, type CheckResponse } from './api.js';
import { getSiteHandler, type SiteHandler } from './sites/index.js';

const OVERLAY_ID = 'aegis-overlay';
const BLOCK_MESSAGE_ID = 'aegis-block-message';

let isChecking = false;
let siteHandler: SiteHandler | null = null;

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
      <div class="aegis-icon">&#9888;</div>
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
 * Show service unavailable message.
 */
function showServiceUnavailable(): void {
  removeOverlay();

  const overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'aegis-overlay aegis-offline';
  overlay.innerHTML = `
    <div class="aegis-overlay-content">
      <div class="aegis-icon">&#128268;</div>
      <div class="aegis-title">Aegis Unavailable</div>
      <div class="aegis-text">The safety service is not running. Prompt was allowed.</div>
      <button class="aegis-dismiss">OK</button>
    </div>
  `;

  const dismissBtn = overlay.querySelector('.aegis-dismiss');
  dismissBtn?.addEventListener('click', removeOverlay);

  const container = siteHandler?.getOverlayContainer() || document.body;
  container.appendChild(overlay);

  // Auto-dismiss after 3 seconds
  setTimeout(removeOverlay, 3000);
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
async function interceptPrompt(prompt: string): Promise<{ blocked: boolean; response?: CheckResponse }> {
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
    console.warn('[Aegis] Service unavailable:', error);
    removeOverlay();
    showServiceUnavailable();
    return { blocked: false }; // Allow on service failure
  } finally {
    isChecking = false;
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
  interceptPrompt(prompt).then(({ blocked, response }) => {
    if (!blocked) {
      // Allow - re-trigger the submission
      submitOriginal(input);
    } else if (response && shouldWarn(response.action)) {
      // Show warning with option to proceed
      showWarningOverlay(response, () => submitOriginal(input));
    } else if (response) {
      // Blocked - show message and clear input
      showBlockedOverlay(response);
      siteHandler?.clearInput(input);
    }
  });
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
    interceptPrompt(prompt).then(({ blocked, response }) => {
      if (!blocked) {
        submitOriginal(input);
      } else if (response && shouldWarn(response.action)) {
        showWarningOverlay(response, () => submitOriginal(input));
      } else if (response) {
        showBlockedOverlay(response);
        siteHandler?.clearInput(input);
      }
    });
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

  console.log(`[Aegis] Intercepting input on ${siteHandler?.name}`);
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

  console.log(`[Aegis] Intercepting button on ${siteHandler?.name}`);
}

/**
 * Initialize interception for the current page.
 */
function initialize(): void {
  siteHandler = getSiteHandler();

  if (!siteHandler) {
    console.log('[Aegis] No handler for this site');
    return;
  }

  console.log(`[Aegis] Initializing on ${siteHandler.name}`);

  // Set up interception on existing elements
  const inputs = siteHandler.findInputs();
  const buttons = siteHandler.findSubmitButtons();

  inputs.forEach(setupInputInterception);
  buttons.forEach(btn => setupButtonInterception(btn, inputs));

  // Watch for dynamically added elements
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
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initialize);
} else {
  initialize();
}
