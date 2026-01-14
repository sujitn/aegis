/**
 * Aegis popup script.
 * Displays connection status and troubleshooting help.
 */

interface StatusResponse {
  status: 'online' | 'offline' | 'error';
  lastCheck: number;
  activeTabs: number;
  consecutiveFailures?: number;
  nextRetryIn?: number | null;
}

const elements = {
  statusIndicator: document.getElementById('status-indicator')!,
  statusText: document.getElementById('status-text')!,
  onlineView: document.getElementById('online-view')!,
  offlineView: document.getElementById('offline-view')!,
  lastCheckOnline: document.getElementById('last-check-online')!,
  offlineDescription: document.getElementById('offline-description')!,
  retryText: document.getElementById('retry-text')!,
  refreshBtn: document.getElementById('refresh-btn')!,
  refreshBtnText: document.getElementById('refresh-btn-text')!,
  failModeToggle: document.getElementById('fail-mode-toggle') as HTMLInputElement,
};

/**
 * Format a timestamp as relative time.
 */
function formatRelativeTime(timestamp: number): string {
  if (!timestamp) return '--';

  const now = Date.now();
  const diff = now - timestamp;

  if (diff < 60000) {
    return 'Just now';
  } else if (diff < 3600000) {
    const minutes = Math.floor(diff / 60000);
    return `${minutes}m ago`;
  } else if (diff < 86400000) {
    const hours = Math.floor(diff / 3600000);
    return `${hours}h ago`;
  } else {
    const days = Math.floor(diff / 86400000);
    return `${days}d ago`;
  }
}

/**
 * Format milliseconds as human readable.
 */
function formatRetryTime(ms: number): string {
  if (ms < 60000) {
    return `${Math.ceil(ms / 1000)} seconds`;
  } else {
    return `${Math.ceil(ms / 60000)} minute(s)`;
  }
}

/**
 * Update the UI based on service status.
 */
function updateUI(response: StatusResponse): void {
  const isOnline = response.status === 'online';

  // Status indicator
  elements.statusIndicator.className = `status-indicator ${isOnline ? 'online' : 'offline'}`;
  elements.statusText.textContent = isOnline ? 'Online' : 'Offline';

  // Show appropriate view
  elements.onlineView.classList.toggle('hidden', !isOnline);
  elements.offlineView.classList.toggle('hidden', isOnline);

  if (isOnline) {
    elements.lastCheckOnline.textContent = formatRelativeTime(response.lastCheck);
  } else {
    // Update retry info
    if (response.nextRetryIn) {
      elements.retryText.textContent = `Retrying in ${formatRetryTime(response.nextRetryIn)}...`;
    } else {
      elements.retryText.textContent = 'Retrying automatically...';
    }

    // Update description based on failure count
    if (response.consecutiveFailures && response.consecutiveFailures > 3) {
      elements.offlineDescription.textContent = 'Connection failed multiple times';
    } else {
      elements.offlineDescription.textContent = 'Cannot connect to Aegis';
    }
  }
}

/**
 * Refresh the status.
 */
async function refreshStatus(): Promise<void> {
  elements.refreshBtnText.textContent = 'Checking...';
  elements.refreshBtn.setAttribute('disabled', 'true');

  try {
    // Use FORCE_RETRY to reset backoff and check immediately
    const response = await chrome.runtime.sendMessage({ type: 'FORCE_RETRY' });
    updateUI(response);
  } catch (error) {
    updateUI({ status: 'offline', lastCheck: 0, activeTabs: 0 });
  } finally {
    elements.refreshBtnText.textContent = 'Check Connection';
    elements.refreshBtn.removeAttribute('disabled');
  }
}

/**
 * Load fail mode setting from storage.
 */
async function loadFailModeSetting(): Promise<void> {
  try {
    const result = await chrome.storage.local.get(['failMode']);
    // Default is 'closed' (fail-safe on), toggle checked = closed
    elements.failModeToggle.checked = result.failMode !== 'open';
  } catch {
    elements.failModeToggle.checked = true; // Default to fail-safe on
  }
}

/**
 * Handle fail mode toggle change.
 */
async function handleFailModeToggle(): Promise<void> {
  const failMode = elements.failModeToggle.checked ? 'closed' : 'open';
  await chrome.storage.local.set({ failMode });
}

/**
 * Initialize the popup.
 */
async function initialize(): Promise<void> {
  // Get current status
  try {
    const response = await chrome.runtime.sendMessage({ type: 'GET_STATUS' });
    updateUI(response);
  } catch (error) {
    updateUI({ status: 'offline', lastCheck: 0, activeTabs: 0 });
  }

  // Load fail mode setting
  await loadFailModeSetting();

  // Set up event listeners
  elements.refreshBtn.addEventListener('click', refreshStatus);
  elements.failModeToggle.addEventListener('change', handleFailModeToggle);
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', initialize);
