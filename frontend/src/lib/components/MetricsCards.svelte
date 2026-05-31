<script lang="ts">
	import { api, type MetricsResponse } from '$lib/api';
	import { createResource } from '$lib/resource.svelte';

	import Panel from './Panel.svelte';

	const res = createResource<MetricsResponse>(api.metrics, 10_000);

	$effect(() => {
		res.start();
		return res.stop;
	});

	// beszel's `info` keys are version-specific and not guaranteed — render every
	// primitive entry as a labelled stat rather than hard-coding a schema. Numbers
	// are rounded; the raw key is shown so it's legible even before we pin the map.
	function stats(info: Record<string, unknown> | null): [string, string][] {
		if (!info) return [];
		return Object.entries(info)
			.filter(([, v]) => typeof v === 'number' || typeof v === 'string')
			.map(([k, v]) => [k, typeof v === 'number' ? round(v) : String(v)]);
	}

	function round(n: number): string {
		return Number.isInteger(n) ? String(n) : n.toFixed(1);
	}
</script>

<Panel title="Host & container metrics" loading={res.loading && !res.data} error={res.error}>
	{#if res.data}
		{#if res.data.systems.length === 0}
			<p class="muted">No systems reported.</p>
		{:else}
			<div class="systems">
				{#each res.data.systems as sys (sys.name)}
					<div class="system">
						<div class="head">
							<span class="dot" class:up={sys.status === 'up'}></span>
							<span class="sysname">{sys.name}</span>
							{#if sys.host}<span class="host">{sys.host}</span>{/if}
						</div>
						<div class="chips">
							{#each stats(sys.info) as [k, v] (k)}
								<span class="chip"><b>{k}</b>{v}</span>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</Panel>

<style>
	.systems {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.system {
		background: var(--halo-bg-light);
		border: 1px solid var(--halo-border);
		border-radius: var(--halo-radius);
		padding: 0.8rem;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		margin-bottom: 0.5rem;
	}
	.dot {
		width: 0.7rem;
		height: 0.7rem;
		border-radius: 50%;
		background: var(--halo-disconnected);
		flex: none;
	}
	.dot.up {
		background: var(--halo-connected);
	}
	.sysname {
		font-weight: 600;
	}
	.host {
		color: var(--halo-text-muted);
		font-size: 0.85em;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}
	.chip {
		display: inline-flex;
		gap: 0.35em;
		padding: 0.2rem 0.5rem;
		background: var(--halo-accent-soft);
		border-radius: var(--halo-radius-pill);
		font-size: 0.8em;
		font-variant-numeric: tabular-nums;
	}
	.chip b {
		color: var(--halo-text-muted);
		font-weight: 500;
	}
	.muted {
		color: var(--halo-text-muted);
		margin: 0;
	}
</style>
