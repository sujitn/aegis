/**
 * Aegis background service worker.
 * Manages connection to local Aegis service with automatic reconnection.
 */

import { checkServiceStatus, type ServiceStatus } from './api.js';

interface State {
  status: ServiceStatus;
  lastCheck: number;
  activeTabs: Set<number>;
  consecutiveFailures: number;
  retryTimeout: number | null;
}

const state: State = {
  status: 'offline',
  lastCheck: 0,
  activeTabs: new Set(),
  consecutiveFailures: 0,
  retryTimeout: null,
};

// Retry intervals (exponential backoff)
const RETRY_INTERVALS = [
  5000,    // 5 seconds
  10000,   // 10 seconds
  30000,   // 30 seconds
  60000,   // 1 minute
  120000,  // 2 minutes
];

const ONLINE_CHECK_INTERVAL = 30000; // 30 seconds when online

/**
 * Update the extension icon based on service status.
 */
function updateIcon(status: ServiceStatus): void {
  const iconPath = status === 'online'
    ? {
        16: 'icons/icon16.png',
        32: 'icons/icon32.png',
        48: 'icons/icon48.png',
        128: 'icons/icon128.png',
      }
    : {
        16: 'icons/icon16-offline.png',
        32: 'icons/icon32-offline.png',
        48: 'icons/icon48-offline.png',
        128: 'icons/icon128-offline.png',
      };

  chrome.action.setIcon({ path: iconPath }).catch(() => {
    // Fallback: just use the default icon
  });

  // Update badge
  if (status === 'online') {
    chrome.action.setBadgeText({ text: '' });
  } else {
    chrome.action.setBadgeText({ text: '!' });
    chrome.action.setBadgeBackgroundColor({ color: '#f44336' });
  }

  // Update title
  const title = status === 'online'
    ? 'Aegis AI Safety - Protection Active'
    : 'Aegis AI Safety - Service Offline (Click for help)';
  chrome.action.setTitle({ title });
}

/**
 * Get the next retry interval based on consecutive failures.
 */
function getRetryInterval(): number {
  const index = Math.min(state.consecutiveFailures, RETRY_INTERVALS.length - 1);
  return RETRY_INTERVALS[index];
}

/**
 * Schedule the next status check.
 */
function scheduleNextCheck(): void {
  if (state.retryTimeout) {
    clearTimeout(state.retryTimeout);
  }

  const interval = state.status === 'online'
    ? ONLINE_CHECK_INTERVAL
    : getRetryInterval();

  state.retryTimeout = setTimeout(checkStatus, interval) as unknown as number;
}

/**
 * Check the service status and update state.
 */
async function checkStatus(): Promise<void> {
  const previousStatus = state.status;
  const result = await checkServiceStatus();

  state.status = result.status;
  state.lastCheck = Date.now();

  if (result.status === 'online') {
    state.consecutiveFailures = 0;
  } else {
    state.consecutiveFailures++;
  }

  updateIcon(state.status);

  // Store status for popup
  await chrome.storage.local.set({
    serviceStatus: state.status,
    lastCheck: state.lastCheck,
    consecutiveFailures: state.consecutiveFailures,
  });

  // Notify content scripts if status changed
  if (previousStatus !== state.status) {
    notifyContentScripts(state.status);
  }

  // Schedule next check
  scheduleNextCheck();
}

/**
 * Notify all content scripts of status change.
 */
function notifyContentScripts(status: ServiceStatus): void {
  chrome.tabs.query({}, (tabs) => {
    for (const tab of tabs) {
      if (tab.id && state.activeTabs.has(tab.id)) {
        chrome.tabs.sendMessage(tab.id, { type: 'STATUS_CHANGED', status }).catch(() => {
          // Tab might not have content script
        });
      }
    }
  });
}

/**
 * Handle messages from content scripts and popup.
 */
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  switch (message.type) {
    case 'CONTENT_LOADED':
      if (sender.tab?.id) {
        state.activeTabs.add(sender.tab.id);
      }
      sendResponse({ status: state.status });
      break;

    case 'GET_STATUS':
      sendResponse({
        status: state.status,
        lastCheck: state.lastCheck,
        activeTabs: state.activeTabs.size,
        consecutiveFailures: state.consecutiveFailures,
        nextRetryIn: state.status === 'offline' ? getRetryInterval() : null,
      });
      break;

    case 'CHECK_STATUS':
      checkStatus().then(() => {
        sendResponse({ status: state.status });
      });
      return true; // Will respond asynchronously

    case 'FORCE_RETRY':
      // Immediate retry requested by user
      state.consecutiveFailures = 0; // Reset backoff
      checkStatus().then(() => {
        sendResponse({ status: state.status });
      });
      return true;

    default:
      sendResponse({ error: 'Unknown message type' });
  }
});

/**
 * Handle tab removal.
 */
chrome.tabs.onRemoved.addListener((tabId) => {
  state.activeTabs.delete(tabId);
});

/**
 * Initialize the background worker.
 */
async function initialize(): Promise<void> {
  // Initial status check
  await checkStatus();
}

// Start initialization
initialize();
