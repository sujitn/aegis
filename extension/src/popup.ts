/**
 * Aegis popup script.
 */

interface StatusResponse {
  status: 'online' | 'offline' | 'error';
  lastCheck: number;
  activeTabs: number;
}

const elements = {
  statusIndicator: document.getElementById('status-indicator')!,
  statusText: document.getElementById('status-text')!,
  statusIcon: document.getElementById('status-icon')!,
  statusMessage: document.getElementById('status-message')!,
  statusDescription: document.getElementById('status-description')!,
  lastCheck: document.getElementById('last-check')!,
  connectionError: document.getElementById('connection-error')!,
  refreshBtn: document.getElementById('refresh-btn')!,
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
 * Update the UI based on service status.
 */
function updateUI(response: StatusResponse): void {
  const isOnline = response.status === 'online';

  // Status indicator
  elements.statusIndicator.className = `status-indicator ${isOnline ? 'online' : 'offline'}`;
  elements.statusText.textContent = isOnline ? 'Online' : 'Offline';

  // Status icon
  elements.statusIcon.className = `status-icon ${isOnline ? 'online' : 'offline'}`;
  elements.statusIcon.textContent = isOnline ? '\u2713' : '!';

  // Status message
  if (isOnline) {
    elements.statusMessage.textContent = 'Protection Active';
    elements.statusDescription.textContent = 'AI prompts are being filtered for safety';
  } else {
    elements.statusMessage.textContent = 'Service Unavailable';
    elements.statusDescription.textContent = 'The Aegis service is not running';
  }

  // Last check time
  elements.lastCheck.textContent = formatRelativeTime(response.lastCheck);

  // Connection error
  elements.connectionError.classList.toggle('hidden', isOnline);
}

/**
 * Refresh the status.
 */
async function refreshStatus(): Promise<void> {
  elements.refreshBtn.textContent = 'Checking...';
  elements.refreshBtn.setAttribute('disabled', 'true');

  try {
    const response = await chrome.runtime.sendMessage({ type: 'CHECK_STATUS' });
    updateUI(response);
  } catch (error) {
    console.error('[Aegis] Failed to check status:', error);
    updateUI({ status: 'offline', lastCheck: 0, activeTabs: 0 });
  } finally {
    elements.refreshBtn.textContent = 'Refresh Status';
    elements.refreshBtn.removeAttribute('disabled');
  }
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
    console.error('[Aegis] Failed to get status:', error);
    updateUI({ status: 'offline', lastCheck: 0, activeTabs: 0 });
  }

  // Set up refresh button
  elements.refreshBtn.addEventListener('click', refreshStatus);
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', initialize);
