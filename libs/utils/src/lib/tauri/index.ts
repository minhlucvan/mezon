/**
 * Tauri Desktop App Bridge
 *
 * This module provides a `window.electron` compatible API for Tauri.
 * It will be dynamically injected at build time for the Tauri desktop app,
 * allowing the frontend to use the same API regardless of Electron or Tauri.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

// Store event listeners for cleanup
const eventListeners: Map<string, Map<Function, UnlistenFn>> = new Map();

/**
 * Get the current platform in Electron format
 */
function getPlatform(): NodeJS.Platform {
	const ua = navigator.userAgent.toLowerCase();
	if (ua.includes('win')) return 'win32';
	if (ua.includes('mac')) return 'darwin';
	if (ua.includes('linux')) return 'linux';
	return 'linux';
}

/**
 * Tauri implementation of the Electron bridge API
 * Exposes the same interface as window.electron from Electron preload
 */
export const tauriElectronBridge = {
	platform: getPlatform(),

	getAppVersion: async (): Promise<string> => {
		const version = await invoke<string>('get_app_version');
		return version;
	},

	on: (eventName: string, callback: (...args: any[]) => void): void => {
		// Map Electron event names to Tauri events
		const tauriEventName = mapEventName(eventName);

		listen(tauriEventName, (event) => {
			callback(event, event.payload);
		}).then((unlisten) => {
			if (!eventListeners.has(eventName)) {
				eventListeners.set(eventName, new Map());
			}
			eventListeners.get(eventName)!.set(callback, unlisten);
		});
	},

	send: (eventName: string, ...params: any[]): void => {
		// Map Electron event names to Tauri commands
		const payload = params.length === 1 ? params[0] : params;
		emit(mapEventName(eventName), payload);
	},

	removeListener: (channel: string, listener: Function): void => {
		const listeners = eventListeners.get(channel);
		if (listeners) {
			const unlisten = listeners.get(listener);
			if (unlisten) {
				unlisten();
				listeners.delete(listener);
			}
		}
	},

	getDeviceId: async (): Promise<string> => {
		return invoke<string>('get_device_id');
	},

	senderId: async (_senderId: string): Promise<string> => {
		return invoke<string>('get_sender_id');
	},

	setBadgeCount: (badgeCount: number | null): void => {
		invoke('set_badge_count', { count: badgeCount ?? 0 });
	},

	onWindowBlurred: (callback: () => void): void => {
		listen('window-blurred', () => callback());
	},

	onWindowFocused: (callback: () => void): void => {
		listen('window-focused', () => callback());
	},

	onNotificationClick: (callback: (data: any) => void): void => {
		listen('notification-clicked', (event) => callback(event.payload));
	},

	invoke: async (channel: string, data?: any): Promise<any> => {
		// Map Electron channels to Tauri commands
		const command = mapChannelToCommand(channel);
		return invoke(command, data);
	},

	openImageWindow: async (props: any, _options?: any, _params?: Record<string, string>): Promise<void> => {
		return invoke('open_image_window', { options: props });
	},

	handleActionShowImage: async (action: string, url: string): Promise<any> => {
		return invoke('handle_action_show_image', {
			action: action,
			url: url
		});
	},

	dowloadImage: async (url: string): Promise<void> => {
		return invoke('download_file', {
			options: { url, filename: null }
		});
	},

	getScreenSources: async (
		source: string
	): Promise<{ sources: { id: string; name: string; thumbnail: string; icon: string }[]; total: number; hasMore: boolean }> => {
		return invoke('get_screen_sources', { source });
	},

	loadMoreScreenSources: async (
		source: string,
		offset: number
	): Promise<{ sources: { id: string; name: string; thumbnail: string; icon: string }[]; hasMore: boolean }> => {
		return invoke('load_more_screen_sources', { source, offset });
	},

	clearScreenSourcesCache: async (source?: string): Promise<{ success: boolean }> => {
		return invoke('clear_screen_sources_cache', { source });
	},

	setRatioWindow: (ratio: boolean): void => {
		invoke('set_ratio_window', { ratio: ratio ? 1.0 : -1.0 });
	},

	launchAppWindow: async (props: string): Promise<void> => {
		// Parse props if it's a JSON string
		return invoke('launch_app_window', { props });
	},

	// Additional clipboard methods for image support
	copyImageToClipboard: async (url: string): Promise<boolean> => {
		return invoke<boolean>('copy_image_to_clipboard', { url });
	},

	getActiveWindow: async (): Promise<{ windowClass: string; windowName: string } | null> => {
		return invoke('get_active_window');
	}
};

/**
 * Map Electron event names to Tauri event names
 */
function mapEventName(electronEvent: string): string {
	const eventMap: Record<string, string> = {
		'window-blurred': 'window-blurred',
		'window-focused': 'window-focused',
		'APP::ACTIVE_WINDOW': 'active-window',
		'APP::TRIGGER_SHORTCUT': 'trigger-shortcut',
		'APP::UPDATE_AVAILABLE': 'update-available',
		'APP::DOWNLOAD_PROGRESS': 'download-progress',
		'APP::UPDATE_ERROR': 'update-error',
		'APP::LOCK_SCREEN': 'lock-screen',
		'APP::UNLOCK_SCREEN': 'unlock-screen',
		'APP::NOTIFICATION_CLICKED': 'notification-clicked'
	};
	return eventMap[electronEvent] || electronEvent;
}

/**
 * Map Electron IPC channels to Tauri commands
 */
function mapChannelToCommand(channel: string): string {
	const commandMap: Record<string, string> = {
		'APP::GET_APP_VERSION': 'get_app_version',
		'APP::GET_DEVICE_ID': 'get_device_id',
		'APP::SENDER_ID': 'get_sender_id',
		'APP::SET_BADGE_COUNT': 'set_badge_count',
		'APP::DOWNLOAD_FILE': 'download_file',
		'APP::CHECK_UPDATE': 'check_update',
		'APP::INSTALL_UPDATE': 'install_update',
		'APP::TITLE_BAR_ACTION': 'title_bar_action',
		'APP::GET_WINDOW_STATE': 'get_window_state',
		'APP::REQUEST_PERMISSION_MICROPHONE': 'request_permission_microphone',
		'APP::REQUEST_PERMISSION_CAMERA': 'request_permission_camera',
		'APP::REQUEST_PERMISSION_SCREEN': 'get_screen_sources',
		'APP::SET_RATIO_WINDOW': 'set_ratio_window',
		'APP::SHOW_NOTIFICATION': 'show_notification',
		'APP::UPDATE_ACTIVITY_TRACKING': 'update_activity_tracking'
	};
	return commandMap[channel] || channel.toLowerCase().replace(/::/g, '_').replace(/^app_/, '');
}

/**
 * Initialize the Tauri bridge by exposing it as window.electron
 * This should be called once when the app starts
 */
export function initTauriBridge(): void {
	if (typeof window !== 'undefined') {
		(window as any).electron = tauriElectronBridge;
	}
}

/**
 * Check if running in Tauri
 */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI__' in window && '__TAURI_INTERNALS__' in window;
}

// Auto-initialize if in Tauri environment
if (isTauri()) {
	initTauriBridge();
}

// TypeScript declarations
declare global {
	interface Window {
		__TAURI__?: typeof import('@tauri-apps/api');
		__TAURI_INTERNALS__?: unknown;
		electron?: typeof tauriElectronBridge;
	}
}
