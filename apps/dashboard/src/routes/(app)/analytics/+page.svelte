<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { api } from '$stores/api';
	import type { SystemStats, HealthCheck, RetentionDistribution } from '$types';

	let data: Record<string, any> | null = $state(null);
	let stats: SystemStats | null = $state(null);
	let health: HealthCheck | null = $state(null);
	let retention: RetentionDistribution | null = $state(null);
	let loading = $state(true);

	// Action feedback
	let dreamStatus: 'idle' | 'running' | 'done' = $state('idle');
	let consolidateStatus: 'idle' | 'running' | 'done' = $state('idle');

	// Computed scores
	let healthScore = $derived.by(() => {
		if (!stats || !data) return 0;
		const retentionScore = (stats.averageRetention ?? 0) * 40;
		const coverageScore = ((stats.embeddingCoverage ?? 0) / 100) * 20;
		const connectionDensity = Math.min(((data.summary?.totalConnections ?? 0) / Math.max(stats.totalMemories, 1)) * 50, 20);
		const dreamBonus = (data.summary?.totalDreams ?? 0) > 0 ? 10 : 0;
		const feedbackBonus = (data.accessLog?.length ?? 0) > 0 ? 10 : 0;
		return Math.round(retentionScore + coverageScore + connectionDensity + dreamBonus + feedbackBonus);
	});

	let healthGrade = $derived(
		healthScore >= 90 ? 'A+' : healthScore >= 80 ? 'A' : healthScore >= 70 ? 'B' :
		healthScore >= 60 ? 'C' : healthScore >= 40 ? 'D' : 'F'
	);

	let healthColor = $derived(
		healthScore >= 80 ? '#10b981' : healthScore >= 60 ? '#f59e0b' : '#ef4444'
	);

	onMount(async () => {
		try {
			const [analyticsData, statsData, healthData, retentionData] = await Promise.all([
				api.analytics(),
				api.stats(),
				api.health(),
				api.retentionDistribution(),
			]);
			data = analyticsData;
			stats = statsData;
			health = healthData;
			retention = retentionData;
		} catch {
			// partial load ok
		} finally {
			loading = false;
		}

		if (data && browser) {
			const { Chart, registerables } = await import('chart.js');
			Chart.register(...registerables);
			Chart.defaults.color = '#8b95a5';
			Chart.defaults.borderColor = 'rgba(139, 92, 246, 0.1)';

			renderMemoryGrowth(Chart);
			renderTypeDistribution(Chart);
			renderRetentionBuckets(Chart);
			renderWorkspaceDistribution(Chart);
			renderAccessActivity(Chart);
			renderConnectionTypes(Chart);
		}
	});

	async function runDream() {
		dreamStatus = 'running';
		try {
			await api.dream();
			dreamStatus = 'done';
			data = await api.analytics();
			setTimeout(() => { dreamStatus = 'idle'; }, 2000);
		} catch { dreamStatus = 'idle'; }
	}

	async function runConsolidation() {
		consolidateStatus = 'running';
		try {
			await api.consolidate();
			consolidateStatus = 'done';
			const [a, s, h, r] = await Promise.all([api.analytics(), api.stats(), api.health(), api.retentionDistribution()]);
			data = a; stats = s; health = h; retention = r;
			setTimeout(() => { consolidateStatus = 'idle'; }, 2000);
		} catch { consolidateStatus = 'idle'; }
	}

	function renderMemoryGrowth(Chart: any) {
		const ctx = document.getElementById('chart-growth') as HTMLCanvasElement;
		if (!ctx || !data?.dailyCreated) return;
		const entries = Object.entries(data.dailyCreated) as [string, number][];
		const last14 = entries.slice(-14);
		new Chart(ctx, {
			type: 'bar',
			data: {
				labels: last14.map(([d]) => d.slice(5)),
				datasets: [{ label: 'Created', data: last14.map(([, c]) => c), backgroundColor: 'rgba(139, 92, 246, 0.6)', borderRadius: 4 }]
			},
			options: { responsive: true, plugins: { legend: { display: false } }, scales: { y: { beginAtZero: true, ticks: { stepSize: 1 } } } }
		});
	}

	function renderTypeDistribution(Chart: any) {
		const ctx = document.getElementById('chart-types') as HTMLCanvasElement;
		if (!ctx || !data?.typeDistribution) return;
		const entries = Object.entries(data.typeDistribution) as [string, number][];
		const colors = ['#00A8FF', '#9D00FF', '#FFB800', '#00FFD1', '#FF3CAC', '#FF4757', '#8B95A5'];
		new Chart(ctx, {
			type: 'doughnut',
			data: { labels: entries.map(([t]) => t), datasets: [{ data: entries.map(([, c]) => c), backgroundColor: colors, borderWidth: 0 }] },
			options: { responsive: true, plugins: { legend: { position: 'right', labels: { boxWidth: 10, padding: 6, font: { size: 10 } } } }, cutout: '65%' }
		});
	}

	function renderRetentionBuckets(Chart: any) {
		const ctx = document.getElementById('chart-retention') as HTMLCanvasElement;
		if (!ctx || !data?.retentionBuckets) return;
		const labels = Object.keys(data.retentionBuckets);
		const values = Object.values(data.retentionBuckets) as number[];
		const colors = ['#ef4444', '#f59e0b', '#eab308', '#22c55e', '#10b981'];
		new Chart(ctx, {
			type: 'bar',
			data: { labels, datasets: [{ data: values, backgroundColor: colors, borderRadius: 4 }] },
			options: { responsive: true, plugins: { legend: { display: false } }, scales: { y: { beginAtZero: true, ticks: { stepSize: 1 } } } }
		});
	}

	function renderWorkspaceDistribution(Chart: any) {
		const ctx = document.getElementById('chart-workspaces') as HTMLCanvasElement;
		if (!ctx || !data?.workspaceDistribution) return;
		const entries = Object.entries(data.workspaceDistribution) as [string, number][];
		if (entries.length === 0) return;
		const colors = ['#6366f1', '#a855f7', '#ec4899', '#f59e0b', '#10b981', '#06b6d4'];
		new Chart(ctx, {
			type: 'bar',
			data: { labels: entries.map(([w]) => w), datasets: [{ data: entries.map(([, c]) => c), backgroundColor: colors, borderRadius: 4 }] },
			options: { responsive: true, indexAxis: 'y', plugins: { legend: { display: false } }, scales: { x: { beginAtZero: true, ticks: { stepSize: 1 } } } }
		});
	}

	function renderAccessActivity(Chart: any) {
		const ctx = document.getElementById('chart-activity') as HTMLCanvasElement;
		if (!ctx || !data?.accessLog || data.accessLog.length === 0) return;
		const log = data.accessLog as { date: string; type: string; count: number }[];
		const dates = [...new Set(log.map(l => l.date))].sort().slice(-14);
		const types = [...new Set(log.map(l => l.type))];
		const colorMap: Record<string, string> = { search_hit: '#6366f1', promote: '#10b981', demote: '#ef4444' };
		new Chart(ctx, {
			type: 'bar',
			data: {
				labels: dates.map(d => d.slice(5)),
				datasets: types.map(t => ({ label: t, data: dates.map(d => log.find(l => l.date === d && l.type === t)?.count ?? 0), backgroundColor: colorMap[t] || '#8b95a5', borderRadius: 2 }))
			},
			options: { responsive: true, plugins: { legend: { labels: { boxWidth: 10, font: { size: 10 } } } }, scales: { x: { stacked: true }, y: { stacked: true, beginAtZero: true } } }
		});
	}

	function renderConnectionTypes(Chart: any) {
		const ctx = document.getElementById('chart-connections') as HTMLCanvasElement;
		if (!ctx || !data?.connectionStats?.byType) return;
		const types = data.connectionStats.byType as { type: string; count: number }[];
		if (types.length === 0) return;
		const colors = ['#8b5cf6', '#fbbf24', '#ef4444', '#10b981', '#38bdf8', '#a855f7'];
		new Chart(ctx, {
			type: 'doughnut',
			data: { labels: types.map(t => t.type), datasets: [{ data: types.map(t => t.count), backgroundColor: colors, borderWidth: 0 }] },
			options: { responsive: true, plugins: { legend: { position: 'right', labels: { boxWidth: 10, padding: 6, font: { size: 10 } } } }, cutout: '65%' }
		});
	}
</script>

<div class="p-6 max-w-6xl mx-auto space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-xl text-bright font-semibold">Command Center</h1>
		{#if health}
			<div class="flex items-center gap-2">
				<div class="w-2.5 h-2.5 rounded-full animate-pulse" style="background: {health.status === 'healthy' ? '#10b981' : health.status === 'degraded' ? '#f59e0b' : '#ef4444'}"></div>
				<span class="text-xs text-dim">{health.status.toUpperCase()} · v{health.version}</span>
			</div>
		{/if}
	</div>

	{#if loading}
		<div class="grid grid-cols-4 gap-4">
			{#each Array(4) as _}
				<div class="h-28 glass-subtle rounded-xl animate-pulse"></div>
			{/each}
		</div>
	{:else if data && stats}

		<!-- Health Score + Key Metrics -->
		<div class="grid grid-cols-5 gap-3">
			<!-- Health Score (big) -->
			<div class="glass rounded-xl p-5 flex flex-col items-center justify-center row-span-2"
				style="border-color: {healthColor}20">
				<div class="text-4xl font-black" style="color: {healthColor}">{healthGrade}</div>
				<div class="text-lg font-bold text-bright">{healthScore}/100</div>
				<div class="text-[10px] text-dim mt-1">Health Score</div>
				<div class="w-full mt-3 h-2 bg-white/[0.04] rounded-full overflow-hidden">
					<div class="h-full rounded-full transition-all" style="width: {healthScore}%; background: {healthColor}"></div>
				</div>
			</div>

			<div class="glass-subtle rounded-xl p-4 text-center">
				<div class="text-2xl text-bright font-bold">{stats.totalMemories}</div>
				<div class="text-[10px] text-dim mt-1">Total Memories</div>
			</div>
			<div class="glass-subtle rounded-xl p-4 text-center">
				<div class="text-2xl font-bold" style="color: {stats.averageRetention > 0.7 ? '#10b981' : stats.averageRetention > 0.4 ? '#f59e0b' : '#ef4444'}">{(stats.averageRetention * 100).toFixed(0)}%</div>
				<div class="text-[10px] text-dim mt-1">Avg Retention</div>
			</div>
			<div class="glass-subtle rounded-xl p-4 text-center">
				<div class="text-2xl text-synapse-glow font-bold">{data.summary.totalConnections}</div>
				<div class="text-[10px] text-dim mt-1">Connections</div>
			</div>
			<div class="glass-subtle rounded-xl p-4 text-center">
				<div class="text-2xl text-dream-glow font-bold">{data.summary.totalDreams}</div>
				<div class="text-[10px] text-dim mt-1">Dream Cycles</div>
			</div>

			<!-- Second row of metrics -->
			<div class="glass-subtle rounded-xl p-3 flex items-center gap-2">
				<span>📌</span>
				<div><div class="text-sm text-bright font-semibold">{data.summary.coreMemories}</div><div class="text-[9px] text-dim">Pinned</div></div>
			</div>
			<div class="glass-subtle rounded-xl p-3 flex items-center gap-2">
				<span>♾️</span>
				<div><div class="text-sm text-bright font-semibold">{data.summary.permanentMemories}</div><div class="text-[9px] text-dim">Permanent</div></div>
			</div>
			<div class="glass-subtle rounded-xl p-3 flex items-center gap-2">
				<span>📊</span>
				<div><div class="text-sm text-bright font-semibold">{Object.keys(data.workspaceDistribution).length}</div><div class="text-[9px] text-dim">Workspaces</div></div>
			</div>
			<div class="glass-subtle rounded-xl p-3 flex items-center gap-2">
				<span>🔗</span>
				<div><div class="text-sm text-bright font-semibold">{stats.embeddingCoverage?.toFixed(0) ?? 0}%</div><div class="text-[9px] text-dim">Embeddings</div></div>
			</div>
		</div>

		<!-- Quick Actions -->
		<div class="flex gap-3 flex-wrap">
			<button onclick={runDream}
				disabled={dreamStatus === 'running'}
				class="px-4 py-2 rounded-xl text-sm transition
					{dreamStatus === 'done' ? 'bg-green-500/20 text-green-300 border border-green-500/40' :
					 dreamStatus === 'running' ? 'bg-dream/10 text-dream-glow/50 animate-pulse border border-dream/20' :
					 'bg-dream/20 border border-dream/40 text-dream-glow hover:bg-dream/30'}">
				{dreamStatus === 'done' ? '✓ Dream Complete' : dreamStatus === 'running' ? '◈ Dreaming...' : '◈ Run Dream'}
			</button>
			<button onclick={runConsolidation}
				disabled={consolidateStatus === 'running'}
				class="px-4 py-2 rounded-xl text-sm transition
					{consolidateStatus === 'done' ? 'bg-green-500/20 text-green-300 border border-green-500/40' :
					 consolidateStatus === 'running' ? 'bg-amber-500/10 text-amber-300/50 animate-pulse border border-amber-500/20' :
					 'bg-amber-500/20 border border-amber-500/40 text-amber-300 hover:bg-amber-500/30'}">
				{consolidateStatus === 'done' ? '✓ Consolidated' : consolidateStatus === 'running' ? '⟳ Running...' : '⟳ Run Consolidation'}
			</button>
			<div class="ml-auto text-xs text-dim self-center">
				{#if stats.dueForReview > 0}
					<span class="text-amber-400">{stats.dueForReview} due for review</span>
				{:else}
					<span class="text-green-400">All memories up to date</span>
				{/if}
			</div>
		</div>

		<!-- Charts -->
		<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
			<div class="glass-subtle rounded-xl p-4">
				<h3 class="text-sm text-bright font-medium mb-3">Memory Growth (14d)</h3>
				<canvas id="chart-growth" height="180"></canvas>
			</div>
			<div class="glass-subtle rounded-xl p-4">
				<h3 class="text-sm text-bright font-medium mb-3">Retention Health</h3>
				<canvas id="chart-retention" height="180"></canvas>
			</div>
			<div class="glass-subtle rounded-xl p-4">
				<h3 class="text-sm text-bright font-medium mb-3">Memory Types</h3>
				<canvas id="chart-types" height="180"></canvas>
			</div>
			<div class="glass-subtle rounded-xl p-4">
				<h3 class="text-sm text-bright font-medium mb-3">Connection Types</h3>
				<canvas id="chart-connections" height="180"></canvas>
			</div>
			<div class="glass-subtle rounded-xl p-4">
				<h3 class="text-sm text-bright font-medium mb-3">Workspaces</h3>
				<canvas id="chart-workspaces" height="180"></canvas>
			</div>
			<div class="glass-subtle rounded-xl p-4">
				<h3 class="text-sm text-bright font-medium mb-3">Activity (searches · promotes · demotes)</h3>
				<canvas id="chart-activity" height="180"></canvas>
			</div>
		</div>

		<!-- Endangered Memories -->
		{#if retention && retention.endangered.length > 0}
			<div class="glass rounded-xl p-5" style="border-color: rgba(239,68,68,0.15)">
				<h3 class="text-sm text-red-400 font-semibold mb-3">⚠ Endangered Memories ({retention.endangered.length})</h3>
				<div class="space-y-2 max-h-48 overflow-y-auto">
					{#each retention.endangered.slice(0, 15) as m}
						<div class="flex items-center gap-3 text-sm">
							<span class="text-xs font-mono w-10 text-right" style="color: {(m.retention ?? 0) > 0.3 ? '#f59e0b' : '#ef4444'}">{((m.retention ?? 0) * 100).toFixed(0)}%</span>
							<span class="text-dim truncate flex-1">{m.content}</span>
							<span class="text-[9px] text-muted shrink-0">{m.nodeType}</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Dream History -->
		{#if data.dreamHistory && data.dreamHistory.length > 0}
			<div class="glass-subtle rounded-xl p-5">
				<h3 class="text-sm text-bright font-medium mb-3">Dream History</h3>
				<div class="overflow-x-auto">
					<table class="w-full text-xs">
						<thead>
							<tr class="text-dim border-b border-white/[0.06]">
								<th class="text-left py-2 px-2">Date</th>
								<th class="text-right py-2 px-2">Replayed</th>
								<th class="text-right py-2 px-2">Connections</th>
								<th class="text-right py-2 px-2">Insights</th>
							</tr>
						</thead>
						<tbody>
							{#each data.dreamHistory as dream}
								<tr class="border-b border-white/[0.03] hover:bg-white/[0.02]">
									<td class="py-1.5 px-2 text-text">{new Date(dream.date).toLocaleString()}</td>
									<td class="py-1.5 px-2 text-right text-dim">{dream.memoriesReplayed}</td>
									<td class="py-1.5 px-2 text-right text-synapse-glow">{dream.connectionsFound}</td>
									<td class="py-1.5 px-2 text-right text-dream-glow">{dream.insightsGenerated}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>
		{/if}

		<!-- Retention Distribution (detailed bars from stats page) -->
		{#if retention}
			<div class="glass-subtle rounded-xl p-5">
				<h3 class="text-sm text-bright font-medium mb-4">Retention Distribution (detailed)</h3>
				<div class="flex items-end gap-1 h-32">
					{#each retention.distribution as bucket, i}
						{@const maxCount = Math.max(...retention.distribution.map((b: any) => b.count), 1)}
						{@const height = (bucket.count / maxCount) * 100}
						{@const color = i < 3 ? '#ef4444' : i < 5 ? '#f59e0b' : i < 7 ? '#10b981' : '#6366f1'}
						<div class="flex-1 flex flex-col items-center gap-1">
							<span class="text-[10px] text-dim">{bucket.count}</span>
							<div class="w-full rounded-t transition-all duration-500" style="height: {height}%; background: {color}; opacity: 0.7; min-height: 2px"></div>
							<span class="text-[9px] text-muted">{bucket.range}</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

	{:else}
		<div class="text-center py-20 text-dim">
			<p>No data available yet. Start using Vestige to generate insights.</p>
		</div>
	{/if}
</div>
