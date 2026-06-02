<script lang="ts">
	import { api, type MetricsResponse, type SystemMetrics } from '$lib/api';
	import { createResource } from '$lib/resource.svelte';

	import Panel from './Panel.svelte';

	const res = createResource<MetricsResponse>(api.metrics, 10_000);

	$effect(() => {
		res.start();
		return res.stop;
	});

	// beszel's `info` blob uses terse keys (its SystemInfo struct). We surface the
	// human-useful ones as table columns; deprecated/internal keys (h/k/c/m/p/b,
	// ct connection-type enum) are ignored.
	const asNum = (v: unknown): number | null => (typeof v === 'number' ? v : null);

	function load(v: unknown): string | null {
		return Array.isArray(v)
			? v.map((n) => (typeof n === 'number' ? n.toFixed(2) : '?')).join(' ')
			: null;
	}
	function net(v: unknown): string | null {
		const n = asNum(v);
		if (n === null) return null;
		if (n < 1024) return `${n} B/s`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB/s`;
		return `${(n / (1024 * 1024)).toFixed(1)} MB/s`;
	}
	function temp(v: unknown): string | null {
		const n = asNum(v);
		return n === null ? null : `${n.toFixed(1)} °C`;
	}
	function uptime(v: unknown): string | null {
		const s = asNum(v);
		if (s === null) return null;
		const d = Math.floor(s / 86400);
		const h = Math.floor((s % 86400) / 3600);
		return d > 0 ? `${d} days` : `${h}h ${Math.floor((s % 3600) / 60)}m`;
	}
	function agent(v: unknown): string | null {
		return typeof v === 'string' ? `v${v}` : null;
	}

	// Percent columns render value + a mini bar; text columns just format a value.
	const PCT_COLS = [
		{ key: 'cpu', label: 'CPU' },
		{ key: 'mp', label: 'Memory' },
		{ key: 'dp', label: 'Disk' },
		{ key: 'g', label: 'GPU' }
	] as const;
	const TEXT_COLS: { key: string; label: string; fmt: (v: unknown) => string | null }[] = [
		{ key: 'la', label: 'Load', fmt: load },
		{ key: 'bb', label: 'Net', fmt: net },
		{ key: 'dt', label: 'Temp', fmt: temp },
		{ key: 'u', label: 'Uptime', fmt: uptime },
		{ key: 'v', label: 'Agent', fmt: agent }
	];

	const info = (s: SystemMetrics) => s.info ?? {};
	const pctOf = (s: SystemMetrics, key: string) => asNum(info(s)[key]);
</script>

<Panel title="Host & container metrics" loading={res.loading && !res.data} error={res.error}>
	{#if res.data}
		{#if res.data.systems.length === 0}
			<p class="muted">No systems reported.</p>
		{:else}
			{@const beszel = res.data.beszel_url}
			<div class="scroll">
				<table>
					<thead>
						<tr>
							<th class="sys">System</th>
							{#each PCT_COLS as col (col.key)}<th>{col.label}</th>{/each}
							{#each TEXT_COLS as col (col.key)}<th>{col.label}</th>{/each}
						</tr>
					</thead>
					<tbody>
						{#each res.data.systems as sys (sys.name)}
							{@const sysUrl =
								beszel && sys.id ? `${beszel}/system/${encodeURIComponent(sys.id)}` : null}
							<!-- Whole row clickable for the mouse; the cell's <a> covers keyboard/right-click. -->
							<tr
								class:clickable={sysUrl}
								onclick={() => {
									if (sysUrl) window.location.href = sysUrl;
								}}
							>
								<td class="sys">
									<span class="dot" class:up={sys.status === 'up'}></span>
									{#if sysUrl}
										<a class="name link" href={sysUrl} rel="external">{sys.name}</a>
									{:else}
										<span class="name">{sys.name}</span>
									{/if}
								</td>
								{#each PCT_COLS as col (col.key)}
									{@const n = pctOf(sys, col.key)}
									<td>
										{#if n !== null}
											<div class="pct">
												<span class="val">{n.toFixed(1)}%</span>
												<span class="bar"
													><span style="width:{Math.max(0, Math.min(100, n))}%"></span></span
												>
											</div>
										{/if}
									</td>
								{/each}
								{#each TEXT_COLS as col (col.key)}
									{@const v = col.fmt(info(sys)[col.key])}
									<td class="num">{v ?? ''}</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	{/if}
</Panel>

<style>
	.scroll {
		overflow-x: auto;
		border: 1px solid var(--halo-border);
		border-radius: var(--halo-radius);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.85rem;
	}
	th {
		text-align: left;
		font-weight: 500;
		color: var(--halo-text-muted);
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 0.6rem 0.6rem 0.5rem;
		white-space: nowrap;
	}
	td {
		padding: 0.6rem 0.6rem;
		border-top: 1px solid var(--halo-border);
		white-space: nowrap;
		vertical-align: middle;
	}
	/* Inset the edge columns from the box border. */
	th:first-child,
	td:first-child {
		padding-left: 0.8rem;
	}
	th:last-child,
	td:last-child {
		padding-right: 0.8rem;
	}
	td.sys {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
	}
	.dot {
		width: 0.6rem;
		height: 0.6rem;
		border-radius: 50%;
		background: var(--halo-disconnected);
		flex: none;
	}
	.dot.up {
		background: var(--halo-connected);
	}
	tr.clickable {
		cursor: pointer;
	}
	tr.clickable:hover {
		background: var(--halo-accent-soft);
	}
	.link {
		color: inherit;
		text-decoration: none;
	}
	tr.clickable:hover .link {
		color: var(--halo-accent);
	}
	.num {
		font-variant-numeric: tabular-nums;
	}
	.pct {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-variant-numeric: tabular-nums;
	}
	.val {
		min-width: 3.2em;
	}
	.bar {
		display: inline-block;
		width: 3.5rem;
		height: 0.45rem;
		border-radius: var(--halo-radius-pill);
		background: var(--halo-off-bg);
		overflow: hidden;
		flex: none;
	}
	.bar span {
		display: block;
		height: 100%;
		background: var(--halo-accent);
	}
	.muted {
		color: var(--halo-text-muted);
		margin: 0;
	}
</style>
