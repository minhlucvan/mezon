/**
 * Tauri Desktop App Utilities
 *
 * This module provides utilities for detecting and interacting with Tauri.
 * Use these functions when you need to conditionally run code based on the runtime environment.
 */

/**
 * Check if the app is running inside Tauri
 */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI__' in window && '__TAURI_INTERNALS__' in window;
}

/**
 * Get the Tauri API if available
 */
export function getTauriAPI(): typeof window.__TAURI__ | null {
	if (isTauri()) {
		return window.__TAURI__;
	}
	return null;
}

/**
 * Invoke a Tauri command
 */
export async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
	if (!isTauri()) {
		console.warn(`Tauri command "${cmd}" called outside of Tauri environment`);
		return null;
	}

	try {
		const { invoke } = await import('@tauri-apps/api/core');
		return await invoke<T>(cmd, args);
	} catch (error) {
		console.error(`Tauri command "${cmd}" failed:`, error);
		throw error;
	}
}

/**
 * Get the app version from Tauri
 */
export async function getTauriAppVersion(): Promise<string | null> {
	return invokeTauri<string>('get_app_version');
}

/**
 * Get the current platform
 */
export async function getTauriPlatform(): Promise<string | null> {
	return invokeTauri<string>('get_platform');
}

/**
 * Set the badge count on the dock/taskbar icon
 */
export async function setTauriBadgeCount(count: number): Promise<void> {
	await invokeTauri('set_badge_count', { count });
}

/**
 * Clear the badge count
 */
export async function clearTauriBadge(): Promise<void> {
	await invokeTauri('clear_badge');
}

/**
 * Show a system notification
 */
export async function showTauriNotification(title: string, body: string): Promise<void> {
	await invokeTauri('show_notification', { title, body });
}

/**
 * Minimize the app to system tray
 */
export async function minimizeTauriToTray(): Promise<void> {
	await invokeTauri('minimize_to_tray');
}

/**
 * Quit the Tauri application
 */
export async function quitTauriApp(): Promise<void> {
	await invokeTauri('quit_app');
}

// ============================================================================
// Native Image Clipboard APIs (matching Electron's clipboard.writeImage)
// ============================================================================

/**
 * Copy image to clipboard from URL
 * Downloads the image and copies it natively to the system clipboard
 * @param url - The URL of the image to copy
 * @returns true if successful
 */
export async function copyTauriImageToClipboard(url: string): Promise<boolean> {
	const result = await invokeTauri<boolean>('copy_image_to_clipboard', { url });
	return result ?? false;
}

/**
 * Copy image to clipboard from base64 data
 * @param base64Data - Base64 encoded image data (with or without data URL prefix)
 * @returns true if successful
 */
export async function copyTauriBase64ImageToClipboard(base64Data: string): Promise<boolean> {
	const result = await invokeTauri<boolean>('copy_base64_image_to_clipboard', { base64Data });
	return result ?? false;
}

/**
 * Copy image to clipboard from raw bytes
 * @param imageData - Raw image bytes (PNG, JPEG, etc.)
 * @returns true if successful
 */
export async function copyTauriImageBytesToClipboard(imageData: Uint8Array): Promise<boolean> {
	const result = await invokeTauri<boolean>('copy_image_bytes_to_clipboard', {
		imageData: Array.from(imageData)
	});
	return result ?? false;
}

/**
 * Read image from clipboard as base64 PNG data URL
 * @returns Base64 PNG data URL or null if no image in clipboard
 */
export async function readTauriImageFromClipboard(): Promise<string | null> {
	const result = await invokeTauri<string | null>('read_image_from_clipboard');
	return result;
}

/**
 * Check if clipboard contains an image
 * @returns true if clipboard contains an image
 */
export async function tauriClipboardHasImage(): Promise<boolean> {
	const result = await invokeTauri<boolean>('clipboard_has_image');
	return result ?? false;
}

// ============================================================================
// Screen Capture APIs (matching Electron's desktopCapturer)
// ============================================================================

export interface ScreenSourceItem {
	id: string;
	name: string;
	thumbnail: string;
	icon: string;
}

export interface ScreenSourcesResponse {
	sources: ScreenSourceItem[];
	total: number;
	hasMore: boolean;
}

export interface LoadMoreSourcesResponse {
	sources: ScreenSourceItem[];
	hasMore: boolean;
}

/**
 * Get screen sources for screen sharing (screens or windows)
 * Matches Electron's desktopCapturer.getSources() API
 */
export async function getTauriScreenSources(source: 'screen' | 'window'): Promise<ScreenSourcesResponse> {
	const result = await invokeTauri<ScreenSourcesResponse>('get_screen_sources', { source });
	return result || { sources: [], total: 0, hasMore: false };
}

/**
 * Load more screen sources with pagination
 */
export async function loadMoreTauriScreenSources(
	source: 'screen' | 'window',
	offset: number
): Promise<LoadMoreSourcesResponse> {
	const result = await invokeTauri<LoadMoreSourcesResponse>('load_more_screen_sources', {
		source,
		offset
	});
	return result || { sources: [], hasMore: false };
}

/**
 * Clear the screen sources cache
 */
export async function clearTauriScreenSourcesCache(source?: 'screen' | 'window'): Promise<{ success: boolean }> {
	const result = await invokeTauri<{ success: boolean }>('clear_screen_sources_cache', { source });
	return result || { success: false };
}

// ============================================================================
// Active Window Detection APIs (matching Electron's mezon-active-windows)
// ============================================================================

export interface ActiveWindowInfo {
	/** The window class or application name (e.g., "Code", "Spotify", "chrome") */
	windowClass: string;
	/** The window title */
	windowName: string;
	/** The executable path (if available) */
	path: string | null;
	/** Process ID */
	pid: number | null;
}

/**
 * Get the currently active/focused window
 * Matches the mezon-active-windows package format used in Electron
 */
export async function getTauriActiveWindow(): Promise<ActiveWindowInfo | null> {
	const result = await invokeTauri<ActiveWindowInfo | null>('get_active_window');
	return result;
}

/**
 * Enable or disable activity tracking
 */
export async function setTauriActivityTracking(enabled: boolean): Promise<void> {
	await invokeTauri('update_activity_tracking', { enabled });
}

// TypeScript declarations for Tauri globals
declare global {
	interface Window {
		__TAURI__?: typeof import('@tauri-apps/api');
		__TAURI_INTERNALS__?: unknown;
	}
}
