/**
 * Aegis background service worker.
 */

import { checkServiceStatus, type ServiceStatus } from './api.js';

interface State {
  status: ServiceStatus;
  lastCheck: number;
  activeTabs: Set<number>;
}

const state: State = {
  status: 'offline',
  lastCheck: 0,
  activeTabs: new Set(),
};

const STATUS_CHECK_INTERVAL = 30000; // 30 seconds

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
}

/**
 * Check the service status and update state.
 */
async function checkStatus(): Promise<void> {
  const result = await checkServiceStatus();
  state.status = result.status;
  state.lastCheck = Date.now();

  updateIcon(state.status);

  // Store status for popup
  await chrome.storage.local.set({
    serviceStatus: state.status,
    lastCheck: state.lastCheck,
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
      });
      break;

    case 'CHECK_STATUS':
      checkStatus().then(() => {
        sendResponse({ status: state.status });
      });
      return true; // Will respond asynchronously

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
  console.log('[Aegis] Background worker starting');

  // Initial status check
  await checkStatus();

  // Set up periodic status checks
  setInterval(checkStatus, STATUS_CHECK_INTERVAL);

  console.log('[Aegis] Background worker ready');
}

// Start initialization
initialize();
