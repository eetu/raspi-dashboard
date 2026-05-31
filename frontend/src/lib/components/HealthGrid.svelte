<script lang="ts">
	import { api, type HealthResponse } from '$lib/api';
	import { createResource } from '$lib/resource.svelte';

	import Panel from './Panel.svelte';

	const res = createResource<HealthResponse>(api.health, 15_000);

	$effect(() => {
		res.start();
		return res.stop;
	});

	function uptimePct(u: number): string {
		return `${(u * 100).toFixed(1)}%`;
	}
</script>

<Panel title="Service health" loading={res.loading && !res.data} error={res.error}>
	{#if res.data}
		<div class="grid">
			{#each res.data.endpoints as e (e.group + '/' + e.name)}
				<div class="cell" class:down={!e.up} title={uptimePct(e.uptime) + ' uptime'}>
					<span class="dot" class:up={e.up}></span>
					<span class="name">{e.name}</span>
					<span class="uptime">{uptimePct(e.uptime)}</span>
				</div>
			{/each}
		</div>
	{/if}
</Panel>

<style>
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 0.5rem;
	}
	.cell {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.6rem 0.8rem;
		background: var(--halo-bg-light);
		border-radius: var(--halo-radius);
		border: 1px solid var(--halo-border);
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
	.name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.uptime {
		color: var(--halo-text-muted);
		font-variant-numeric: tabular-nums;
		font-size: 0.85em;
	}
	.cell.down .name {
		color: var(--halo-error);
	}
</style>
