<script lang="ts">
	import '$lib/styles/halo.css';

	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>raspi-dashboard</title>
</svelte:head>

{@render children()}
