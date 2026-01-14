# Aegis AI Safety - Privacy Policy

**Last Updated:** January 2026

## Overview

Aegis AI Safety is a parental control extension that monitors AI chatbot interactions to protect children from harmful content. This privacy policy explains what data we collect and how we use it.

## Data Collection

### What We Collect

1. **AI Chat Prompts**: Text that users type into supported AI chatbots (ChatGPT, Claude, Gemini)
2. **Classification Results**: Whether prompts were allowed, warned, or blocked
3. **Timestamps**: When interactions occurred

### What We Do NOT Collect

- Personal identification information
- Browsing history outside of supported AI sites
- Passwords or credentials
- Payment information
- Location data
- AI responses (only user prompts are analyzed)

## Data Processing

### Local Processing Only

**All data processing happens locally on your device.**

- Prompts are sent to the local Aegis desktop application (127.0.0.1) for analysis
- No data is sent to external servers
- No data is stored in the cloud
- No data is shared with third parties

### How It Works

1. User types a prompt in an AI chatbot
2. Extension intercepts the prompt before submission
3. Prompt is sent to local Aegis app for safety classification
4. If safe, prompt proceeds normally
5. If harmful, prompt is blocked or warning is shown
6. Parent can review blocked prompts in the local dashboard

## Data Storage

- All logs are stored locally in the Aegis desktop application
- Parents can export or delete logs at any time
- Uninstalling Aegis removes all stored data

## Permissions Explained

| Permission | Why We Need It |
|------------|----------------|
| `storage` | Store extension settings locally |
| `activeTab` | Detect when user is on an AI chatbot site |
| `host_permissions` (AI sites) | Inject content script to monitor prompts |
| `host_permissions` (localhost) | Communicate with local Aegis app |

## Children's Privacy

Aegis is designed to protect children's safety while respecting their privacy:

- Parents see what categories of content were blocked, not full prompt text (configurable)
- No data leaves the local device
- Children can see why their prompts were blocked

## Your Rights

You can:
- View all stored data in the Aegis dashboard
- Export your data as CSV
- Delete all data via the uninstall wizard
- Disable the extension at any time

## Changes to This Policy

We will update this policy as needed. Significant changes will be communicated through the extension update notes.

## Contact

For privacy concerns or questions:
- GitHub Issues: https://github.com/anthropics/aegis/issues
- Email: privacy@aegis-safety.example.com

## Compliance

This extension is designed to comply with:
- COPPA (Children's Online Privacy Protection Act)
- GDPR (General Data Protection Regulation)
- Chrome Web Store Developer Program Policies
