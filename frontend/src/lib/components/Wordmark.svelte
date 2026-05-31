<script lang="ts">
	// raspi-dashboard brand mark: a shield (security) enclosing a heartbeat pulse
	// (health/metrics) — the two questions the page answers. Single-stroke,
	// currentColor + accent, in the family glyph style. See the raspi-design skill.
	let { size = 26 }: { size?: number } = $props();
</script>

<span class="halo-wordmark wordmark">
	<svg
		width={size}
		height={size}
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="1.8"
		stroke-linecap="round"
		stroke-linejoin="round"
		aria-hidden="true"
	>
		<path d="M12 2.5 4.5 5.5v6c0 4.4 3 7.7 7.5 9.5 4.5-1.8 7.5-5.1 7.5-9.5v-6L12 2.5Z" />
		<path class="pulse" d="M5 12.5h3l1.8-3.5 2.6 6 1.8-3.2 1.3 1.2H19" />
	</svg>
	<span>raspi<span class="accent">dash</span></span>
</span>

<style>
	.wordmark {
		display: inline-flex;
		align-items: center;
		gap: 0.5em;
		font-family: var(--halo-font-heading);
		font-size: 1.4rem;
	}
	.pulse {
		stroke: var(--halo-accent);
	}
</style>
