<script lang="ts">
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';

	import { api, type HealthEntry, type HealthResponse } from '$lib/api';
	import { createResource } from '$lib/resource.svelte';

	import Panel from './Panel.svelte';

	const res = createResource<HealthResponse>(api.health, 15_000);

	$effect(() => {
		res.start();
		return res.stop;
	});

	// Reactive clock so the "down Nm" timers advance on their own, not only when a
	// poll lands (or a reload). Ticks every 10s — fine for minute-granular labels.
	let now = $state(Date.now());
	$effect(() => {
		const id = setInterval(() => {
			now = Date.now();
		}, 10_000);
		return () => clearInterval(id);
	});

	// Which cards are expanded — multiple may be open at once. Keyed by group/name.
	const open = new SvelteSet<string>();

	// Group endpoints by their gatus group, preserving first-seen order so the
	// config controls section ordering. Returns [group, entries][].
	const groups = $derived.by<[string, HealthEntry[]][]>(() => {
		const order: string[] = [];
		const by: Record<string, HealthEntry[]> = {};
		for (const e of res.data?.endpoints ?? []) {
			if (!by[e.group]) {
				by[e.group] = [];
				order.push(e.group);
			}
			by[e.group].push(e);
		}
		return order.map((g) => [g, by[g]]);
	});

	function downCount(entries: HealthEntry[]): number {
		return entries.filter((e) => !e.up).length;
	}

	// Explicit per-group expand override. When unset, a group defaults to expanded
	// iff it has a service down; clicking sets an override so the user can collapse
	// a down group (or expand a healthy one) regardless of that default.
	const groupOverride = new SvelteMap<string, boolean>();

	function isGroupExpanded(group: string, down: number): boolean {
		return groupOverride.get(group) ?? down > 0;
	}

	function toggleGroup(group: string, down: number): void {
		groupOverride.set(group, !isGroupExpanded(group, down));
	}

	function keyOf(e: HealthEntry): string {
		return e.group + '/' + e.name;
	}

	function toggle(e: HealthEntry): void {
		const k = keyOf(e);
		if (open.has(k)) open.delete(k);
		else open.add(k);
	}

	function uptimePct(u: number): string {
		return `${(u * 100).toFixed(1)}%`;
	}

	function hhmm(ts: string): string {
		const d = new Date(ts);
		return `${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}`;
	}

	function shortDuration(ms: number): string {
		const s = Math.floor(ms / 1000);
		if (s < 60) return `${s}s`;
		const m = Math.floor(s / 60);
		if (m < 60) return `${m}m`;
		const h = Math.floor(m / 60);
		if (h < 24) return `${h}h${m % 60}m`;
		return `${Math.floor(h / 24)}d${h % 24}h`;
	}

	// Walk the trailing failure streak (newest is last) to find when it went down.
	// Returns null while up. `now` is passed so the value recomputes each poll.
	function downSince(
		latest: HealthEntry['latest'],
		now: number
	): { at: string; ms: number } | null {
		if (latest.length === 0 || latest[latest.length - 1].success) return null;
		let start = latest[latest.length - 1];
		for (let i = latest.length - 1; i >= 0 && !latest[i].success; i--) start = latest[i];
		// Clamp: pi/browser clock skew can make a just-now timestamp read as future.
		return { at: start.timestamp, ms: Math.max(0, now - new Date(start.timestamp).getTime()) };
	}
</script>

<Panel title="Service health" loading={res.loading && !res.data} error={res.error}>
	{#if res.data}
		{#each groups as [group, entries] (group)}
			{@const down = downCount(entries)}
			{@const expanded = isGroupExpanded(group, down)}
			<section class="group">
				<button
					type="button"
					class="group-row"
					class:down={down > 0}
					aria-expanded={expanded}
					onclick={() => toggleGroup(group, down)}
				>
					<span class="agg-pip" class:up={down === 0}></span>
					<span class="group-name">{group}</span>
					<span class="members">
						{#each entries as e (keyOf(e))}
							<span class="member" class:down={!e.up} title={e.up ? 'up' : 'down'}>{e.name}</span>
						{/each}
					</span>
					<span class="strip">
						{#each entries as e (keyOf(e))}
							<span class="pip" class:up={e.up} title="{e.name} — {e.up ? 'up' : 'down'}"></span>
						{/each}
					</span>
					<span class="material-icons-outlined chevron">
						{expanded ? 'expand_less' : 'expand_more'}
					</span>
				</button>
				{#if expanded}
					<div class="grid">
						{#each entries as e (keyOf(e))}
							{@const since = downSince(e.latest, now)}
							{@const ticks = e.latest}
							{@const last = e.latest.at(-1)}
							<div class="cell" class:expanded={open.has(keyOf(e))}>
								<button
									type="button"
									class="head"
									aria-expanded={open.has(keyOf(e))}
									onclick={() => toggle(e)}
								>
									<span class="dot" class:up={e.up}></span>
									<span class="name">{e.name}</span>
									{#if since}
										<span class="badge">down {shortDuration(since.ms)}</span>
									{/if}
									<span class="material-icons-outlined chevron">
										{open.has(keyOf(e)) ? 'expand_less' : 'expand_more'}
									</span>
								</button>

								{#if open.has(keyOf(e))}
									<div class="detail">
										<div class="ticks">
											{#each ticks as t (t.timestamp)}
												<span
													class="tick"
													class:fail={!t.success}
													title="{hhmm(t.timestamp)} · {t.duration_ms}ms · {t.success
														? 'ok'
														: 'down'}"
												></span>
											{/each}
										</div>
										<div class="meta">
											<span>uptime {uptimePct(e.uptime)}</span>
											{#if last}<span>last {last.duration_ms}ms</span>{/if}
											{#if since}<span class="meta-down">down since {hhmm(since.at)}</span>{/if}
										</div>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{/each}
	{/if}
</Panel>

<style>
	.group {
		margin-bottom: 0.5rem;
		border: 1px solid var(--halo-border);
		border-radius: var(--halo-radius);
		overflow: hidden;
	}
	.group:last-child {
		margin-bottom: 0;
	}
	.group-row {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		width: 100%;
		padding: 0.55rem 0.8rem;
		background: none;
		border: none;
		font: inherit;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.group-row:hover {
		background: var(--halo-accent-soft);
	}
	.group-name {
		flex: none;
		font-family: var(--halo-font-heading);
		font-weight: 500;
		min-width: 5rem;
	}
	.group-row.down .group-name {
		color: var(--halo-error);
	}
	/* Single aggregate indicator — wide screens only. */
	.agg-pip {
		flex: none;
		width: 0.6rem;
		height: 0.6rem;
		border-radius: 50%;
		background: var(--halo-disconnected);
	}
	.agg-pip.up {
		background: var(--halo-connected);
	}
	.members {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		flex: 1;
		font-size: 0.85rem;
		color: var(--halo-text-muted);
	}
	.member.down {
		color: var(--halo-error);
		font-weight: 500;
	}
	.member:not(:last-child)::after {
		content: '·';
		margin-left: 0.4rem;
		color: var(--halo-text-light);
	}
	/* Per-service pip strip — narrow screens only (hidden on wide). */
	.strip {
		display: none;
		flex-wrap: wrap;
		gap: 0.3rem;
		flex: 1;
	}
	.pip {
		width: 0.55rem;
		height: 0.55rem;
		border-radius: 50%;
		background: var(--halo-disconnected);
	}
	.pip.up {
		background: var(--halo-connected);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 0.5rem;
		padding: 0.6rem 0.8rem;
		border-top: 1px solid var(--halo-border);
		align-items: start;
	}
	.cell {
		background: var(--halo-bg-light);
		border-radius: var(--halo-radius);
		border: 1px solid var(--halo-border);
		overflow: hidden;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		width: 100%;
		padding: 0.6rem 0.8rem;
		background: none;
		border: none;
		font: inherit;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.head:hover {
		background: var(--halo-accent-soft);
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
	.badge {
		flex: none;
		font-size: 0.7rem;
		font-variant-numeric: tabular-nums;
		color: var(--halo-bg-main);
		background: var(--halo-disconnected);
		padding: 0.1rem 0.35rem;
		border-radius: var(--halo-radius-pill);
	}
	.chevron {
		flex: none;
		font-size: 1.1rem;
		color: var(--halo-text-muted);
	}
	.detail {
		padding: 0.6rem 0.8rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		border-top: 1px solid var(--halo-border);
	}
	.ticks {
		display: flex;
		gap: 2px;
		height: 1.4rem;
	}
	.tick {
		flex: 1;
		min-width: 3px;
		border-radius: var(--halo-radius-pill);
		background: var(--halo-connected);
	}
	.tick.fail {
		background: var(--halo-disconnected);
	}
	.meta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.8rem;
		color: var(--halo-text-muted);
		font-variant-numeric: tabular-nums;
		font-size: 0.8rem;
	}
	.meta-down {
		color: var(--halo-error);
	}

	/* Narrow: drop the inline names + aggregate pip, show the per-service strip. */
	@media (max-width: 640px) {
		.agg-pip,
		.members {
			display: none;
		}
		.strip {
			display: flex;
		}
	}
</style>
