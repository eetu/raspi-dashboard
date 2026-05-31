<script lang="ts">
	import { api, type CveResponse } from '$lib/api';
	import { createResource } from '$lib/resource.svelte';

	import Panel from './Panel.svelte';

	const res = createResource<CveResponse>(api.cve, 30_000);

	let scanning = $state(false);
	let scanError = $state<string | null>(null);

	$effect(() => {
		res.start();
		return res.stop;
	});

	function ago(hours: number | null): string {
		if (hours === null) return 'unknown';
		if (hours < 1) return 'under an hour ago';
		if (hours < 48) return `${hours}h ago`;
		return `${Math.round(hours / 24)}d ago`;
	}

	// Trigger an out-of-band scan, then poll /api/cve until scanned_at advances
	// (the backend returns 202; the scan runs in trivy's own unit). Caps at ~5min
	// so a stuck scan doesn't spin forever.
	async function scanNow() {
		scanning = true;
		scanError = null;
		const before = res.data?.scanned_at ?? '';
		try {
			await api.requestScan();
			for (let i = 0; i < 60; i++) {
				await new Promise((r) => setTimeout(r, 5000));
				await res.refresh();
				if (res.data && res.data.scanned_at !== before) break;
			}
		} catch (e) {
			scanError = e instanceof Error ? e.message : String(e);
		} finally {
			scanning = false;
		}
	}

	function sevClass(sev: string): string {
		return sev.toUpperCase() === 'CRITICAL' ? 'crit' : 'high';
	}
</script>

<Panel title="CVE scan" loading={res.loading && !res.data} error={res.error}>
	{#snippet actions()}
		<button onclick={scanNow} disabled={scanning}>
			{scanning ? 'Scanning…' : 'Scan now'}
		</button>
	{/snippet}

	{#if res.data}
		<div class="summary">
			<span class="totals">
				<b class="crit">{res.data.total_critical}</b> critical ·
				<b class="high">{res.data.total_high}</b> high
			</span>
			<span class="when" class:stale={res.data.stale}>
				scanned {ago(res.data.age_hours)}{res.data.stale ? ' · stale' : ''}
			</span>
		</div>

		{#if scanError}<p class="err">{scanError}</p>{/if}

		<div class="images">
			{#each res.data.images as img (img.image)}
				<details open={img.critical + img.high > 0}>
					<summary>
						<span class="img-name">{img.image}</span>
						<span class="counts">
							{#if img.critical > 0}<b class="crit">{img.critical}C</b>{/if}
							{#if img.high > 0}<b class="high">{img.high}H</b>{/if}
							{#if img.critical + img.high === 0}<span class="clean">clean</span>{/if}
						</span>
					</summary>
					{#if img.vulns.length > 0}
						<ul>
							{#each img.vulns as v (v.id + v.pkg)}
								<li>
									<span class={'sev ' + sevClass(v.severity)}>{v.severity}</span>
									<code>{v.id}</code>
									<span class="pkg">{v.pkg}</span>
									<span class="title">{v.title}</span>
								</li>
							{/each}
						</ul>
					{/if}
				</details>
			{/each}
		</div>
	{/if}
</Panel>

<style>
	button {
		font-family: var(--halo-font-body);
		font-weight: 600;
		color: var(--halo-bg-main);
		background: var(--halo-accent);
		border: none;
		border-radius: var(--halo-radius-pill);
		padding: 0.45rem 0.9rem;
		cursor: pointer;
		transition: opacity var(--halo-d-fast);
	}
	button:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.summary {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 1rem;
		flex-wrap: wrap;
	}
	.totals {
		font-size: 1.05rem;
	}
	.when {
		color: var(--halo-text-muted);
		font-size: 0.85em;
	}
	.when.stale {
		color: var(--halo-error);
	}
	.crit {
		color: var(--halo-disconnected);
	}
	.high {
		color: var(--halo-accent);
	}
	.clean {
		color: var(--halo-connected);
		font-size: 0.85em;
	}
	.images {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	details {
		background: var(--halo-bg-light);
		border: 1px solid var(--halo-border);
		border-radius: var(--halo-radius);
		padding: 0.5rem 0.7rem;
	}
	summary {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		cursor: pointer;
	}
	.img-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.counts {
		display: flex;
		gap: 0.5rem;
		flex: none;
		font-variant-numeric: tabular-nums;
	}
	ul {
		margin: 0.6rem 0 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	li {
		display: flex;
		gap: 0.5rem;
		align-items: baseline;
		flex-wrap: wrap;
		font-size: 0.85em;
	}
	.sev {
		font-weight: 600;
		font-size: 0.75em;
		padding: 0.1rem 0.35rem;
		border-radius: var(--halo-radius-pill);
	}
	.sev.crit {
		color: var(--halo-disconnected);
		background: rgba(244, 67, 54, 0.12);
	}
	.sev.high {
		color: var(--halo-accent);
		background: var(--halo-accent-soft);
	}
	code {
		font-family: ui-monospace, monospace;
	}
	.pkg {
		color: var(--halo-text-main);
	}
	.title {
		color: var(--halo-text-muted);
	}
	.err {
		color: var(--halo-error);
		margin: 0;
	}
</style>
