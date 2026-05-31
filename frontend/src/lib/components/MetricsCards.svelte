<script lang="ts">
	import { api, type MetricsResponse, type SystemMetrics } from '$lib/api';
	import { createResource } from '$lib/resource.svelte';

	import Panel from './Panel.svelte';

	const res = createResource<MetricsResponse>(api.metrics, 10_000);

	$effect(() => {
		res.start();
		return res.stop;
	});

	// beszel's `info` blob uses terse keys (its SystemInfo struct). Map the
	// human-useful ones to labels + units; everything else (deprecated fields,
	// internal enums like connection type) is dropped rather than shown raw.
	type Stat = { label: string; value: string };

	const asNum = (v: unknown): number | null => (typeof v === 'number' ? v : null);
	const pct = (v: unknown): string | null => {
		const n = asNum(v);
		return n === null ? null : `${n.toFixed(1)}%`;
	};

	function bytesPerSec(v: unknown): string | null {
		const n = asNum(v);
		if (n === null) return null;
		if (n < 1024) return `${n} B/s`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB/s`;
		return `${(n / (1024 * 1024)).toFixed(1)} MB/s`;
	}

	function uptime(v: unknown): string | null {
		const s = asNum(v);
		if (s === null) return null;
		const d = Math.floor(s / 86400);
		const h = Math.floor((s % 86400) / 3600);
		const m = Math.floor((s % 3600) / 60);
		return d > 0 ? `${d}d ${h}h` : h > 0 ? `${h}h ${m}m` : `${m}m`;
	}

	// Ordered: the fields worth a glance first.
	const FIELDS: { key: string; label: string; fmt: (v: unknown) => string | null }[] = [
		{ key: 'cpu', label: 'CPU', fmt: pct },
		{ key: 'mp', label: 'Memory', fmt: pct },
		{ key: 'dp', label: 'Disk', fmt: pct },
		{ key: 'g', label: 'GPU', fmt: pct },
		{
			key: 'dt',
			label: 'Temp',
			fmt: (v) => (asNum(v) === null ? null : `${asNum(v)!.toFixed(1)} °C`)
		},
		{ key: 'bb', label: 'Network', fmt: bytesPerSec },
		{ key: 'u', label: 'Uptime', fmt: uptime },
		{ key: 't', label: 'Threads', fmt: (v) => (asNum(v) === null ? null : String(asNum(v))) },
		{
			key: 'la',
			label: 'Load',
			fmt: (v) =>
				Array.isArray(v)
					? v.map((n) => (typeof n === 'number' ? n.toFixed(2) : '?')).join(' / ')
					: null
		},
		{ key: 'v', label: 'Agent', fmt: (v) => (typeof v === 'string' ? `v${v}` : null) }
	];

	function stats(sys: SystemMetrics): Stat[] {
		const info = sys.info ?? {};
		const out: Stat[] = [];
		for (const f of FIELDS) {
			const value = f.fmt(info[f.key]);
			if (value !== null && value !== '') out.push({ label: f.label, value });
		}
		return out;
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
						<dl class="grid">
							{#each stats(sys) as stat (stat.label)}
								<div class="stat">
									<dt>{stat.label}</dt>
									<dd>{stat.value}</dd>
								</div>
							{/each}
						</dl>
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
		margin-bottom: 0.75rem;
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
	.grid {
		margin: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(90px, 1fr));
		gap: 0.6rem 0.8rem;
	}
	.stat {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}
	dt {
		color: var(--halo-text-muted);
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	dd {
		margin: 0;
		font-variant-numeric: tabular-nums;
		font-size: 0.95rem;
	}
	.muted {
		color: var(--halo-text-muted);
		margin: 0;
	}
</style>
