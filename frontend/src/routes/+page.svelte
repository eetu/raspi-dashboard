<script lang="ts">
	import CvePanel from '$lib/components/CvePanel.svelte';
	import HealthGrid from '$lib/components/HealthGrid.svelte';
	import MetricsCards from '$lib/components/MetricsCards.svelte';
	import Wordmark from '$lib/components/Wordmark.svelte';
</script>

<main>
	<header class="top">
		<Wordmark />
		<span class="tagline">is everything ok, and is anything vulnerable?</span>
	</header>

	<div class="panels">
		<HealthGrid />
		<MetricsCards />
		<CvePanel />
	</div>
</main>

<style>
	main {
		max-width: 880px;
		margin: 0 auto;
		padding: 2rem 1.25rem 4rem;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}
	.top {
		display: flex;
		align-items: baseline;
		gap: 1rem;
		flex-wrap: wrap;
	}
	.tagline {
		color: var(--halo-text-muted);
		font-size: 0.9rem;
	}
	.panels {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
</style>
