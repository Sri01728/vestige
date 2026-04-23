<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$stores/api';
	import { NODE_TYPE_COLORS } from '$types';
	import type { Memory } from '$types';

	// Trail state — each step is a memory + its connections
	interface TrailStep {
		memory: Memory;
		connections: { id: string; content: string; nodeType: string; retention: number; score: number }[];
	}

	let trail: TrailStep[] = $state([]);
	let searchQuery = $state('');
	let searchResults: Memory[] = $state([]);

	// Deep explore state
	let deepMode = $state(false);
	let deepResults: { hop: number; id: string; content: string; nodeType: string; retention: number; tags: string[] }[] = $state([]);
	let deepByHop: Record<string, any[]> = $state({});
	let deepTotal = $state(0);
	let deepSourceMemory: Memory | null = $state(null);
	let maxHops = $state(3);
	let searchOpen = $state(false);
	let loading = $state(false);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;


	async function deepExplore(memory: Memory) {
		searchOpen = false;
		searchQuery = '';
		deepMode = true;
		deepSourceMemory = memory;
		trail = [];
		loading = true;
		try {
			const res = await api.explore(memory.id, 'deep', undefined, 50, maxHops);
			deepResults = ((res.results || []) as any[]).map(r => ({
				hop: r.hop,
				id: r.id,
				content: r.content,
				nodeType: r.nodeType,
				retention: r.retention ?? 0,
				tags: r.tags ?? [],
			}));
			deepByHop = (res.byHop || {}) as Record<string, any[]>;
			deepTotal = (res.totalFound as number) || 0;
		} catch {
			deepResults = [];
			deepByHop = {};
			deepTotal = 0;
		} finally {
			loading = false;
		}
	}

	function onSearchInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		const q = searchQuery.trim();
		if (!q) { searchResults = []; searchOpen = false; return; }
		debounceTimer = setTimeout(async () => {
			try {
				const res = await api.search(q, 8);
				searchResults = res.results;
				searchOpen = searchResults.length > 0;
			} catch { searchResults = []; searchOpen = false; }
		}, 250);
	}

	async function startTrail(memory: Memory) {
		searchOpen = false;
		searchQuery = '';
		trail = [];
		await exploreNode(memory);
	}

	async function exploreNode(memory: Memory) {
		loading = true;
		try {
			const res = await api.explore(memory.id, 'associations', undefined, 10);
			const connections = ((res.results || []) as any[]).map(r => ({
				id: r.id,
				content: r.content,
				nodeType: r.nodeType,
				retention: r.retention ?? 0,
				score: r.score ?? 0,
				connectionType: r.connectionType ?? null,
				source: r.source ?? 'semantic',
			}));
			// Don't add duplicates to trail
			trail = [...trail, { memory, connections }];
		} catch {
			trail = [...trail, { memory, connections: [] }];
		} finally {
			loading = false;
		}
	}

	async function followConnection(conn: { id: string; content: string; nodeType: string; retention: number; connectionType?: string; source?: string }) {
		// Check if already in trail
		if (trail.some(s => s.memory.id === conn.id)) return;
		loading = true;
		try {
			const fullMemory = await api.memories.get(conn.id);
			await exploreNode(fullMemory);
		} catch { /* ignore */ }
		finally { loading = false; }
	}

	function goBackTo(index: number) {
		trail = trail.slice(0, index + 1);
	}

	function clearTrail() {
		trail = [];
		deepMode = false;
		deepResults = [];
		deepByHop = {};
		deepTotal = 0;
		deepSourceMemory = null;
	}


</script>

<div class="p-6 max-w-5xl mx-auto space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-xl text-bright font-semibold">Knowledge Explorer</h1>
		{#if trail.length > 0}
			<button onclick={clearTrail} class="px-3 py-1.5 glass rounded-lg text-xs text-dim hover:text-text transition">
				✕ Clear Trail
			</button>
		{/if}
	</div>

	<!-- Search -->
	<div class="relative">
		<input
			type="text"
			placeholder="Search for a memory to start exploring..."
			bind:value={searchQuery}
			oninput={onSearchInput}
			onkeydown={(e) => { if (e.key === 'Escape') { searchOpen = false; searchQuery = ''; } }}
			class="w-full px-4 py-3 glass rounded-xl text-text text-sm placeholder:text-muted focus:outline-none focus:border-synapse/40 transition"
		/>
		{#if searchOpen && searchResults.length > 0}
			<!-- onmousedown with preventDefault stops the input blur from closing dropdown before click registers -->
			<div
				class="absolute top-full left-0 right-0 mt-1 glass rounded-xl overflow-hidden max-h-72 overflow-y-auto z-20"
				onmousedown={(e) => e.preventDefault()}
			>
				{#each searchResults as result}
					<div class="px-4 py-3 border-b border-white/[0.04] last:border-0 hover:bg-white/[0.04]">
						<div class="flex items-start gap-2.5">
							<div class="w-2 h-2 rounded-full mt-1.5 shrink-0" style="background: {NODE_TYPE_COLORS[result.nodeType] || '#8B95A5'}"></div>
							<div class="flex-1 min-w-0">
								<div class="text-xs text-text line-clamp-2">{result.content}</div>
								<div class="flex gap-2 mt-1 text-[10px] text-muted">
									<span>{result.nodeType}</span>
									<span>{(result.retentionStrength * 100).toFixed(0)}%</span>
								</div>
								<div class="flex gap-2 mt-2">
									<button onclick={() => startTrail(result)}
										class="px-2.5 py-1 rounded-lg text-[10px] bg-synapse/15 text-synapse-glow hover:bg-synapse/25 transition cursor-pointer">
										Trail →
									</button>
									<button onclick={() => deepExplore(result)}
										class="px-2.5 py-1 rounded-lg text-[10px] bg-dream/15 text-dream-glow hover:bg-dream/25 transition cursor-pointer">
										Deep Explore ({maxHops} hops)
									</button>
								</div>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#if deepMode && deepSourceMemory}
		<!-- Deep Explore Results -->
		<div class="space-y-4">
			<!-- Source card -->
			<div class="glass rounded-xl p-4 !border-dream/30">
				<div class="flex items-center gap-2 mb-2">
					<div class="w-2 h-2 rounded-full" style="background: {NODE_TYPE_COLORS[deepSourceMemory.nodeType] || '#8B95A5'}"></div>
					<span class="text-[10px] text-muted uppercase tracking-wider">{deepSourceMemory.nodeType} · origin</span>
				</div>
				<p class="text-sm text-text">{deepSourceMemory.content}</p>
			</div>

			<!-- Hop selector -->
			<div class="flex items-center gap-3">
				<span class="text-xs text-dim">Max hops:</span>
				{#each [1, 2, 3, 4, 5] as h}
					<button
						onclick={() => { maxHops = h; deepExplore(deepSourceMemory); }}
						class="w-7 h-7 rounded-lg text-xs transition
							{maxHops === h ? 'bg-dream/20 text-dream-glow border border-dream/40' : 'glass-subtle text-dim hover:text-text'}"
					>{h}</button>
				{/each}
				<span class="text-xs text-dim ml-auto">{deepTotal} memories found across {Object.keys(deepByHop).length} hops</span>
			</div>

			<!-- Results grouped by hop -->
			{#each Object.entries(deepByHop).sort(([a], [b]) => Number(a) - Number(b)) as [hop, memories]}
				<div>
					<div class="flex items-center gap-2 mb-2">
						<div class="w-6 h-6 rounded-full bg-dream/20 text-dream-glow text-[10px] font-bold flex items-center justify-center">
							{hop}
						</div>
						<span class="text-xs text-dim">Hop {hop} — {memories.length} {memories.length === 1 ? 'memory' : 'memories'}</span>
						<div class="flex-1 h-px bg-white/[0.06]"></div>
					</div>

					<div class="space-y-1.5 ml-8">
						{#each memories as mem}
							<button
								onclick={() => { clearTrail(); startTrail({ id: mem.id, content: mem.content, nodeType: mem.nodeType, retentionStrength: mem.retention, tags: mem.tags || [] } as any); }}
								class="w-full text-left p-3 glass-subtle rounded-xl hover:bg-white/[0.06] transition"
							>
								<div class="flex items-start gap-2">
									<div class="w-2 h-2 rounded-full mt-1.5 shrink-0" style="background: {NODE_TYPE_COLORS[mem.nodeType] || '#8B95A5'}"></div>
									<div class="flex-1 min-w-0">
										<p class="text-xs text-text line-clamp-2">{mem.content}</p>
										<div class="flex gap-2 mt-1 text-[10px] text-muted flex-wrap">
											<span>{mem.nodeType}</span>
											<span>{((mem.retention ?? 0) * 100).toFixed(0)}% ret</span>
											{#each (mem.tags || []).slice(0, 4) as tag}
												<span class="px-1 py-0.5 rounded bg-white/[0.04]">{tag}</span>
											{/each}
										</div>
									</div>
								</div>
							</button>
						{/each}
					</div>
				</div>
			{/each}

			{#if deepTotal === 0 && !loading}
				<div class="text-center py-8 text-dim text-xs">
					No connected memories found within {maxHops} hops. This memory is isolated.
				</div>
			{/if}
		</div>
	{:else if trail.length === 0 && !loading}
		<div class="text-center py-16 text-dim">
			<div class="text-5xl mb-4 opacity-15">◬</div>
			<p class="text-sm">Search for a memory to begin exploring</p>
			<p class="text-xs text-muted mt-2">Click on connections to follow the knowledge trail</p>
		</div>
	{/if}

	<!-- Breadcrumb trail -->
	{#if trail.length > 1}
		<div class="flex items-center gap-1.5 flex-wrap text-xs">
			{#each trail as step, i}
				{#if i > 0}
					<span class="text-synapse/40">→</span>
				{/if}
				<button
					onclick={() => goBackTo(i)}
					class="px-2 py-1 rounded-lg transition truncate max-w-48
						{i === trail.length - 1
							? 'bg-synapse/20 text-synapse-glow border border-synapse/30'
							: 'glass-subtle text-dim hover:text-text'}"
				>
					{step.memory.content.slice(0, 40)}{step.memory.content.length > 40 ? '...' : ''}
				</button>
			{/each}
		</div>
	{/if}

	<!-- Trail steps -->
	{#each trail as step, stepIndex (step.memory.id)}
		{@const isLast = stepIndex === trail.length - 1}

		<div class="relative">
			<!-- Vertical connector line -->
			{#if stepIndex > 0}
				<div class="absolute -top-4 left-6 w-px h-4 bg-synapse/20"></div>
			{/if}

			<!-- Current memory card -->
			<div class="glass rounded-xl p-4 transition-all {isLast ? '!border-synapse/30' : 'opacity-60'}">
				<div class="flex items-start gap-3">
					<!-- Step number -->
					<div class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold shrink-0
						{isLast ? 'bg-synapse/20 text-synapse-glow' : 'bg-white/[0.04] text-dim'}">
						{stepIndex + 1}
					</div>
					<div class="flex-1 min-w-0">
						<div class="flex items-center gap-2 mb-1">
							<div class="w-2 h-2 rounded-full" style="background: {NODE_TYPE_COLORS[step.memory.nodeType] || '#8B95A5'}"></div>
							<span class="text-[10px] text-muted uppercase tracking-wider">{step.memory.nodeType}</span>
							<span class="text-[10px] text-muted">{(step.memory.retentionStrength * 100).toFixed(0)}% retention</span>
						</div>
						<p class="text-sm text-text leading-relaxed">{step.memory.content}</p>
						{#if step.memory.tags.length > 0}
							<div class="flex gap-1.5 mt-2 flex-wrap">
								{#each step.memory.tags.slice(0, 8) as tag}
									<span class="px-1.5 py-0.5 rounded text-[9px] bg-white/[0.04] text-muted">{tag}</span>
								{/each}
							</div>
						{/if}
					</div>
				</div>
			</div>

			<!-- Connections (only show for last step) -->
			{#if isLast && step.connections.length > 0}
				<div class="ml-10 mt-3 space-y-1.5">
					<div class="text-[10px] text-dim uppercase tracking-wider mb-2">
						{step.connections.length} connections — click to follow
					</div>
					{#each step.connections as conn}
						{@const alreadyVisited = trail.some(s => s.memory.id === conn.id)}
						<button
							onclick={() => !alreadyVisited && followConnection(conn)}
							disabled={alreadyVisited}
							class="w-full text-left p-3 rounded-xl transition flex items-start gap-3
								{alreadyVisited
									? 'glass-subtle opacity-30 cursor-not-allowed'
									: 'glass-subtle hover:bg-white/[0.06] hover:border-synapse/20 cursor-pointer'}"
						>
							<div class="flex flex-col items-center gap-0.5 shrink-0 w-10">
								<div class="w-2 h-2 rounded-full" style="background: {NODE_TYPE_COLORS[conn.nodeType] || '#8B95A5'}"></div>
								<span class="text-[9px] text-muted">{(conn.score * 100).toFixed(0)}%</span>
							</div>
							<div class="flex-1 min-w-0">
								<p class="text-xs text-text line-clamp-2">{conn.content}</p>
								<div class="flex gap-2 mt-1 text-[10px] text-muted flex-wrap">
									<span>{conn.nodeType}</span>
									<span>{(conn.retention * 100).toFixed(0)}% ret</span>
									{#if conn.connectionType}
										<span class="px-1.5 py-0.5 rounded bg-synapse/15 text-synapse-glow">{conn.connectionType}</span>
									{/if}
									{#if conn.source === 'graph'}
										<span class="text-green-400">graph edge</span>
									{:else}
										<span class="text-muted">semantic match</span>
									{/if}
									{#if alreadyVisited}<span class="text-synapse-glow">already visited</span>{/if}
								</div>
							</div>
							{#if !alreadyVisited}
								<span class="text-synapse/40 text-sm shrink-0 mt-1">→</span>
							{/if}
						</button>
					{/each}
				</div>
			{:else if isLast && step.connections.length === 0 && !loading}
				<div class="ml-10 mt-3 text-xs text-dim py-4">
					No connections from this memory. This is a leaf node.
				</div>
			{/if}
		</div>
	{/each}

	{#if loading}
		<div class="text-center py-6 text-dim">
			<div class="text-lg animate-pulse mb-2">◎</div>
			<p class="text-xs">Exploring connections...</p>
		</div>
	{/if}

</div>
