<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import {
		getAccounts,
		addAccount,
		removeAccount,
		setAccounts,
	} from "$lib/accounts.svelte";
	import { getAuthUrl, disconnectAccount } from "$lib/api";
	import { loadAccounts, insertAccount, deleteAccount } from "$lib/db";
	import { openUrl } from "@tauri-apps/plugin-opener";
	import { listen, type UnlistenFn } from "@tauri-apps/api/event";
	import type { Account, Provider } from "$lib/types";

	let connecting: Provider | null = $state(null);
	let error: string | null = $state(null);

	const accounts = $derived(getAccounts());
	const googleAccounts = $derived(
		accounts.filter((a) => a.provider === "google"),
	);
	const microsoftAccounts = $derived(
		accounts.filter((a) => a.provider === "microsoft"),
	);

	let unlistenConnected: UnlistenFn | null = null;
	let unlistenError: UnlistenFn | null = null;

	onMount(async () => {
		// Load persisted accounts from SQLite on startup
		try {
			const stored = await loadAccounts();
			setAccounts(stored);
		} catch (e) {
			// SQLite may not be available in dev/test — log but don't crash
			console.warn("Could not load accounts from database:", e);
		}

		// Listen for successful OAuth callbacks from the Rust callback server
		unlistenConnected = await listen<Account>(
			"account-connected",
			async (event) => {
				const account = event.payload;
				try {
					await insertAccount(account);
				} catch (e) {
					console.warn("Could not persist account to database:", e);
				}
				addAccount(account);
				connecting = null;
			},
		);

		// Listen for OAuth errors
		unlistenError = await listen<string>("account-error", (event) => {
			error = `OAuth failed: ${event.payload}`;
			connecting = null;
		});
	});

	onDestroy(() => {
		unlistenConnected?.();
		unlistenError?.();
	});

	async function connect(provider: Provider) {
		error = null;
		connecting = provider;
		try {
			const url = await getAuthUrl(provider);
			// Open the OAuth URL in the system browser via tauri_plugin_opener
			await openUrl(url);
		} catch (e) {
			error = `Failed to start ${provider} connection: ${e}`;
			connecting = null;
		}
	}

	async function disconnect(accountId: string) {
		error = null;
		try {
			await disconnectAccount(accountId);
			// Remove from SQLite (Issue #3: complete credential cleanup)
			try {
				await deleteAccount(accountId);
			} catch (e) {
				console.warn("Could not remove account from database:", e);
			}
			removeAccount(accountId);
		} catch (e) {
			error = `Failed to disconnect: ${e}`;
		}
	}
</script>

<div class="min-h-screen bg-gray-50 p-8 dark:bg-gray-900">
	<div class="mx-auto max-w-2xl">
		<div class="mb-2">
			<a
				href="/"
				class="text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
			>
				← Back
			</a>
		</div>

		<h1 class="mb-6 text-2xl font-bold text-gray-900 dark:text-white">
			Connected Accounts
		</h1>

		{#if error}
			<div
				class="mb-4 rounded-lg bg-red-50 p-4 text-red-700 dark:bg-red-900/20 dark:text-red-400"
			>
				{error}
			</div>
		{/if}

		<!-- Google Section -->
		<section class="mb-8">
			<div class="mb-3 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">
					🔵 Google
				</h2>
				<button
					onclick={() => connect("google")}
					disabled={connecting === "google"}
					class="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
				>
					{connecting === "google" ? "Connecting…" : "Connect Google Account"}
				</button>
			</div>

			{#if googleAccounts.length === 0}
				<p class="text-sm text-gray-500 dark:text-gray-400">
					No Google accounts connected.
				</p>
			{:else}
				<ul class="space-y-2">
					{#each googleAccounts as account (account.id)}
						<li
							class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800"
						>
							<div>
								<p class="font-medium text-gray-900 dark:text-white">
									{account.email}
								</p>
								<p class="text-xs text-gray-500 dark:text-gray-400">
									Connected {new Date(account.connected_at).toLocaleDateString()}
								</p>
							</div>
							<button
								onclick={() => disconnect(account.id)}
								class="rounded px-3 py-1 text-sm text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
							>
								Disconnect
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<!-- Microsoft Section -->
		<section class="mb-8">
			<div class="mb-3 flex items-center justify-between">
				<h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">
					🟦 Microsoft
				</h2>
				<button
					onclick={() => connect("microsoft")}
					disabled={connecting === "microsoft"}
					class="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
				>
					{connecting === "microsoft"
						? "Connecting…"
						: "Connect Microsoft Account"}
				</button>
			</div>

			{#if microsoftAccounts.length === 0}
				<p class="text-sm text-gray-500 dark:text-gray-400">
					No Microsoft accounts connected.
				</p>
			{:else}
				<ul class="space-y-2">
					{#each microsoftAccounts as account (account.id)}
						<li
							class="flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800"
						>
							<div>
								<p class="font-medium text-gray-900 dark:text-white">
									{account.email}
								</p>
								<p class="text-xs text-gray-500 dark:text-gray-400">
									Connected {new Date(account.connected_at).toLocaleDateString()}
								</p>
							</div>
							<button
								onclick={() => disconnect(account.id)}
								class="rounded px-3 py-1 text-sm text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
							>
								Disconnect
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</section>
	</div>
</div>
