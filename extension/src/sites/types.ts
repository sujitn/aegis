/**
 * Common types for site interceptors.
 */

export interface SiteHandler {
  /** Site name for logging */
  name: string;

  /** Check if this handler matches the current URL */
  matches(url: string): boolean;

  /** Find the input element(s) to monitor */
  findInputs(): HTMLElement[];

  /** Find the submit button(s) */
  findSubmitButtons(): HTMLElement[];

  /** Extract the prompt text from the input */
  getPromptText(input: HTMLElement): string;

  /** Clear the input after blocking */
  clearInput(input: HTMLElement): void;

  /** Get the container element to show overlay on */
  getOverlayContainer(): HTMLElement | null;
}

export type InterceptCallback = (prompt: string) => Promise<boolean>;
