// A tiny SWR-ish polling resource built on runes. Fetches immediately, then on
// an interval; exposes reactive data/error/loading. Stops on `stop()` (call it
// from an $effect cleanup). Errors are kept as messages — a failed poll leaves
// the last good `data` in place so a transient upstream blip doesn't blank the
// UI.
export function createResource<T>(fetcher: () => Promise<T>, intervalMs: number) {
	let data = $state<T | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(true);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function refresh() {
		try {
			data = await fetcher();
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	function start() {
		void refresh();
		timer = setInterval(() => void refresh(), intervalMs);
	}

	function stop() {
		if (timer !== null) {
			clearInterval(timer);
			timer = null;
		}
	}

	return {
		get data() {
			return data;
		},
		get error() {
			return error;
		},
		get loading() {
			return loading;
		},
		refresh,
		start,
		stop
	};
}
