<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { api } from '$stores/api';
	import type { TimelineDay } from '$types';
	import { NODE_TYPE_COLORS } from '$types';
	import HeatmapCalendar from '$lib/components/HeatmapCalendar.svelte';

	let timeline: TimelineDay[] = $state([]);
	let loading = $state(true);
	let days = $state(14);
	let expandedDay: string | null = $state(null);

	// Heatmap state — always covers 90 days
	let heatmapData = $state<{ date: string; count: number; types: string[] }[]>([]);
	let heatmapLoading = $state(true);

	onMount(() => {
		loadTimeline();
		loadHeatmap();
	});

	async function loadTimeline() {
		loading = true;
		try {
			const res = await api.timeline(days, 500);
			timeline = res.timeline;
		} catch {
			timeline = [];
		} finally {
			loading = false;
		}
	}

	async function loadHeatmap() {
		heatmapLoading = true;
		try {
			const res = await api.timeline(365, 1000);
			heatmapData = res.timeline.map((day: TimelineDay) => ({
				date: day.date,
				count: day.count,
				types: [...new Set(day.memories.map((m) => m.nodeType))]
			}));
		} catch {
			heatmapData = [];
		} finally {
			heatmapLoading = false;
		}
	}

	function handleHeatmapSelect(date: string) {
		if (!browser) return;

		// Expand the selected day if it exists in the current timeline
		const found = timeline.find((d) => d.date === date);
		if (found) {
			expandedDay = date;
			// Scroll to that day's element
			requestAnimationFrame(() => {
				const el = document.getElementById(`timeline-day-${date}`);
				if (el) {
					el.scrollIntoView({ behavior: 'smooth', block: 'center' });
				}
			});
		} else {
			// Day is outside current range — switch to 90 days and expand
			days = 90;
			expandedDay = date;
			loadTimeline().then(() => {
				requestAnimationFrame(() => {
					const el = document.getElementById(`timeline-day-${date}`);
					if (el) {
						el.scrollIntoView({ behavior: 'smooth', block: 'center' });
					}
				});
			});
		}
	}
</script>

<div class="p-6 max-w-4xl mx-auto space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-xl text-bright font-semibold">Timeline</h1>
		<select bind:value={days} onchange={loadTimeline}
			class="px-3 py-2 bg-white/[0.03] border border-synapse/10 rounded-xl text-dim text-sm focus:outline-none backdrop-blur-sm">
			<option value={7}>7 days</option>
			<option value={14}>14 days</option>
			<option value={30}>30 days</option>
			<option value={90}>90 days</option>
		</select>
	</div>

	<!-- Heatmap Calendar -->
	{#if heatmapLoading}
		<div class="h-32 glass-subtle rounded-xl animate-pulse"></div>
	{:else}
		<HeatmapCalendar data={heatmapData} onselect={handleHeatmapSelect} />
	{/if}

	{#if loading}
		<div class="space-y-4">
			{#each Array(7) as _}
				<div class="h-16 glass-subtle rounded-xl animate-pulse"></div>
			{/each}
		</div>
	{:else if timeline.length === 0}
		<div class="text-center py-20 text-dim">
			<p>No memories in the selected time range.</p>
		</div>
	{:else}
		<div class="relative">
			<!-- Timeline line -->
			<div class="absolute left-6 top-0 bottom-0 w-px bg-synapse/15"></div>

			<div class="space-y-4">
				{#each timeline as day (day.date)}
					<div class="relative pl-14" id="timeline-day-{day.date}">
						<!-- Dot -->
						<div class="absolute left-4 top-3 w-5 h-5 rounded-full border-2 border-synapse bg-void flex items-center justify-center">
							<div class="w-2 h-2 rounded-full bg-synapse"></div>
						</div>

						<button onclick={() => expandedDay = expandedDay === day.date ? null : day.date}
							class="w-full text-left p-4 glass-subtle rounded-xl hover:bg-white/[0.03] transition-all {expandedDay === day.date ? 'ring-1 ring-synapse/30' : ''}">
							<div class="flex items-center justify-between">
								<div>
									<span class="text-sm text-bright font-medium">{day.date}</span>
									<span class="text-xs text-dim ml-2">{day.count} memories</span>
								</div>
								<!-- Dots for memory types -->
								<div class="flex gap-1">
									{#each day.memories.slice(0, 10) as m}
										<div class="w-2 h-2 rounded-full" style="background: {NODE_TYPE_COLORS[m.nodeType] || '#8B95A5'}; opacity: {0.3 + m.retentionStrength * 0.7}"></div>
									{/each}
									{#if day.memories.length > 10}
										<span class="text-xs text-muted">+{day.memories.length - 10}</span>
									{/if}
								</div>
							</div>

							{#if expandedDay === day.date}
								<div class="mt-3 pt-3 border-t border-synapse/10 space-y-2">
									{#each day.memories as m}
										<div class="flex items-start gap-2 text-sm">
											<div class="w-2 h-2 mt-1.5 rounded-full flex-shrink-0" style="background: {NODE_TYPE_COLORS[m.nodeType] || '#8B95A5'}"></div>
											<div class="flex-1 min-w-0">
												<span class="text-dim line-clamp-1">{m.content}</span>
											</div>
											<span class="text-xs text-muted flex-shrink-0">{(m.retentionStrength * 100).toFixed(0)}%</span>
										</div>
									{/each}
								</div>
							{/if}
						</button>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
