// Thin fetch layer over the backend's JSON API. Types are hand-written to match
// the Rust structs (no codegen — see sibling-app). Keep them in sync with
// backend/src/{gatus,beszel,trivy}.rs + routes.rs.

export type StatusResponse = {
	service: string;
	version: string;
	gatus_healthy: boolean;
	beszel_healthy: boolean;
	trivy_scan_age: number | null;
};

export type ResultBrief = {
	success: boolean;
	timestamp: string;
	duration_ms: number;
};

export type HealthEntry = {
	name: string;
	group: string;
	up: boolean;
	uptime: number;
	latest: ResultBrief[];
};

export type HealthResponse = { endpoints: HealthEntry[] };

export type SystemMetrics = {
	name: string;
	status: string;
	host: string | null;
	// beszel's raw latest-stats blob, passed through by the backend. Shape is
	// beszel-version-specific; render defensively.
	info: Record<string, unknown> | null;
};

export type MetricsResponse = { systems: SystemMetrics[] };

export type Vuln = {
	id: string;
	pkg: string;
	severity: string;
	title: string;
};

export type ImageReport = {
	image: string;
	critical: number;
	high: number;
	vulns: Vuln[];
};

export type CveResponse = {
	scanned_at: string;
	images: ImageReport[];
	stale: boolean;
	age_hours: number | null;
	total_critical: number;
	total_high: number;
};

/** Thrown for any non-2xx response; carries the HTTP status. */
export class ApiError extends Error {
	status: number;
	constructor(status: number, message: string) {
		super(message);
		this.status = status;
		this.name = 'ApiError';
	}
}

async function getJSON<T>(path: string): Promise<T> {
	const res = await fetch(path, { headers: { accept: 'application/json' } });
	if (!res.ok) {
		let detail = res.statusText;
		try {
			const body = await res.json();
			if (body && typeof body.detail === 'string') detail = body.detail;
		} catch {
			// non-JSON error body — keep statusText
		}
		throw new ApiError(res.status, detail);
	}
	return (await res.json()) as T;
}

export const api = {
	status: () => getJSON<StatusResponse>('/status'),
	health: () => getJSON<HealthResponse>('/api/health'),
	metrics: () => getJSON<MetricsResponse>('/api/metrics'),
	cve: () => getJSON<CveResponse>('/api/cve'),
	requestScan: async (): Promise<void> => {
		const res = await fetch('/api/cve/scan', { method: 'POST' });
		if (!res.ok) throw new ApiError(res.status, res.statusText);
	}
};
