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

// TypeScript declarations for Tauri globals
declare global {
	interface Window {
		__TAURI__?: typeof import('@tauri-apps/api');
		__TAURI_INTERNALS__?: unknown;
	}
}
