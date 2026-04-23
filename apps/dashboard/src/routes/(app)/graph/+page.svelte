<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { browser } from '$app/environment';
	import { base } from '$app/paths';
	import Graph3D from '$components/Graph3D.svelte';
	import RetentionCurve from '$components/RetentionCurve.svelte';
	import { api } from '$stores/api';
	import type { GraphResponse, GraphNode, GraphEdge, Memory } from '$types';
	import { detectCommunities } from '$lib/graph/community';

	let graphData: GraphResponse | null = $state(null);
	let selectedMemory: Memory | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let isDreaming = $state(false);
	let maxNodes = $state(150);

	// Workspace
	let workspaces: string[] = $state([]);
	let activeWorkspace: string = $state('all');

	// Community detection
	let colorMode: 'type' | 'community' = $state('type');
	let communityMap: Map<string, number> = $state(new Map());

	// Search-as-you-type
	let searchInput = $state('');
	let searchResults: Memory[] = $state([]);
	let searchDropdownOpen = $state(false);
	let highlightIds: string[] = $state([]);
	let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// New controls
	let frozen = $state(false);
	let minConnections = $state(0);
	let infoMessage = $state('');
	let graphComponent: any = $state(null);

	// Button feedback
	let promoteStatus: 'idle' | 'loading' | 'success' | 'error' = $state('idle');
	let demoteStatus: 'idle' | 'loading' | 'success' | 'error' = $state('idle');

	async function handlePromote() {
		if (!selectedMemory) return;
		promoteStatus = 'loading';
		try {
			await api.memories.promote(selectedMemory.id);
			promoteStatus = 'success';
			selectedMemory = await api.memories.get(selectedMemory.id);
			setTimeout(() => { promoteStatus = 'idle'; }, 1500);
		} catch {
			promoteStatus = 'error';
			setTimeout(() => { promoteStatus = 'idle'; }, 2000);
		}
	}

	async function handleDemote() {
		if (!selectedMemory) return;
		demoteStatus = 'loading';
		try {
			await api.memories.demote(selectedMemory.id);
			demoteStatus = 'success';
			selectedMemory = await api.memories.get(selectedMemory.id);
			setTimeout(() => { demoteStatus = 'idle'; }, 1500);
		} catch {
			demoteStatus = 'error';
			setTimeout(() => { demoteStatus = 'idle'; }, 2000);
		}
	}

	onDestroy(() => {
		if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
	});

	let liveNodeCount = $state(0);
	let liveEdgeCount = $state(0);

	let displayNodes = $derived.by((): GraphNode[] => {
		if (!graphData) return [];
		return graphData.nodes;
	});

	let displayEdges = $derived.by((): GraphEdge[] => {
		if (!graphData) return [];
		return graphData.edges;
	});

	onMount(async () => {
		try {
			const tagData = await api.tags();
			workspaces = tagData.workspaces;
		} catch { /* no tags yet */ }
		loadGraph();
	});

	async function loadGraph() {
		loading = true;
		error = '';
		try {
			graphData = await api.graph({
				max_nodes: maxNodes,
				depth: 3,
				workspace: activeWorkspace !== 'all' ? activeWorkspace : undefined,
			});
			if (graphData) {
				liveNodeCount = graphData.nodeCount;
				liveEdgeCount = graphData.edgeCount;
			}
		} catch {
			error = 'No memories yet. Start using Vestige to populate your graph.';
		} finally {
			loading = false;
		}
	}

	async function triggerDream() {
		isDreaming = true;
		try {
			await api.dream();
			await loadGraph();
		} catch { /* dream failed */ }
		finally { isDreaming = false; }
	}

	async function onNodeSelect(nodeId: string) {
		if (!nodeId) { selectedMemory = null; return; }
		try {
			selectedMemory = await api.memories.get(nodeId);
		} catch {
			selectedMemory = null;
		}
	}

	// --- Community Detection ---
	async function toggleColorMode() {
		if (colorMode === 'type') {
			if (graphData && browser) {
				const map = await detectCommunities(displayNodes, displayEdges);
				communityMap = map;
				colorMode = 'community';
			}
		} else {
			communityMap = new Map();
			colorMode = 'type';
		}
	}

	// --- Search-as-you-type ---
	function onSearchInput() {
		if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
		const query = searchInput.trim();
		if (!query) {
			searchResults = [];
			highlightIds = [];
			searchDropdownOpen = false;
			return;
		}
		searchDebounceTimer = setTimeout(async () => {
			if (!browser) return;
			try {
				const result = await api.search(query, 10);
				searchResults = result.results;
				highlightIds = result.results.map((r) => r.id);
				searchDropdownOpen = searchResults.length > 0;
			} catch {
				searchResults = [];
				highlightIds = [];
				searchDropdownOpen = false;
			}
		}, 250);
	}

	function onSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			searchDropdownOpen = false;
			searchResults = [];
			highlightIds = [];
			searchInput = '';
		}
	}

	function selectSearchResult(memory: Memory) {
		searchDropdownOpen = false;
		searchInput = '';
		highlightIds = [memory.id];
		onNodeSelect(memory.id);
		graphComponent?.zoomToNode(memory.id);
	}

	function handleInfoMessage(msg: string) {
		infoMessage = msg;
	}
</script>

<div class="h-full relative" style="isolation: isolate;">
	<!-- Graph layer -->
	<div class="absolute inset-0" style="z-index: 0;">
		{#if loading}
			<div class="h-full flex items-center justify-center">
				<div class="text-center space-y-4">
					<div class="w-16 h-16 mx-auto rounded-full border-2 border-synapse/30 border-t-synapse animate-spin"></div>
					<p class="text-dim text-sm">Loading memory graph...</p>
				</div>
			</div>
		{:else if error}
			<div class="h-full flex items-center justify-center">
				<div class="text-center space-y-4 max-w-md px-8">
					<div class="text-5xl opacity-30">◎</div>
					<h2 class="text-xl text-bright">Your Mind Awaits</h2>
					<p class="text-dim text-sm">{error}</p>
				</div>
			</div>
		{:else if graphData}
			<Graph3D
				bind:this={graphComponent}
				nodes={displayNodes}
				edges={displayEdges}
				centerId={graphData.center_id}
				{isDreaming}
				communityMap={colorMode === 'community' ? communityMap : undefined}
				{highlightIds}
				{frozen}
				{minConnections}
				onSelect={onNodeSelect}
				onInfoMessage={handleInfoMessage}
			/>
		{/if}
	</div>

	<!-- UI overlay -->
	<div class="absolute inset-0 pointer-events-none" style="z-index: 10;">

		<!-- Top controls -->
		<div class="absolute top-4 left-4 right-4 flex items-start gap-3 pointer-events-auto">
			<!-- Search -->
			<div class="relative flex-1 max-w-md">
				<input
					type="text"
					placeholder="Search memories..."
					bind:value={searchInput}
					oninput={onSearchInput}
					onkeydown={onSearchKeydown}
					onfocus={() => { if (searchResults.length > 0) searchDropdownOpen = true; }}
					class="w-full px-4 py-2.5 glass rounded-xl text-text text-sm
						placeholder:text-muted focus:outline-none focus:!border-synapse/40 transition"
				/>
				{#if searchDropdownOpen && searchResults.length > 0}
					<div class="absolute top-full left-0 right-0 mt-1 glass rounded-xl overflow-hidden max-h-80 overflow-y-auto">
						{#each searchResults as result}
							<button
								onclick={() => selectSearchResult(result)}
								class="w-full text-left px-4 py-3 hover:bg-white/[0.06] transition flex items-start gap-3 border-b border-white/[0.04] last:border-b-0"
							>
								<span class="shrink-0 mt-0.5 px-2 py-1 rounded-lg text-[11px] font-semibold uppercase tracking-wider bg-synapse/20 text-synapse-glow">
									{result.nodeType}
								</span>
								<div class="flex-1 min-w-0">
									<div class="text-sm text-text truncate">{result.content.slice(0, 80)}{result.content.length > 80 ? '...' : ''}</div>
									<div class="mt-1.5 flex items-center gap-2">
										<div class="h-1.5 flex-1 bg-white/[0.04] rounded-full overflow-hidden">
											<div class="h-full rounded-full"
												style="width: {result.retentionStrength * 100}%; background: {
													result.retentionStrength > 0.7 ? '#10b981' :
													result.retentionStrength > 0.4 ? '#f59e0b' : '#ef4444'
												}"></div>
										</div>
										<span class="text-xs text-dim shrink-0">{(result.retentionStrength * 100).toFixed(0)}%</span>
									</div>
								</div>
							</button>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Controls row -->
			<div class="flex gap-2 ml-auto flex-wrap justify-end">
				<!-- Workspace selector -->
				{#if workspaces.length > 0}
					<select
						bind:value={activeWorkspace}
						onchange={() => loadGraph()}
						class="px-3 py-2 glass rounded-xl text-xs
							{activeWorkspace !== 'all' ? 'text-synapse-glow border border-synapse/40 bg-synapse/10' : 'text-dim'}"
					>
						<option value="all">All Workspaces</option>
						{#each workspaces as ws}
							<option value={ws}>{ws}</option>
						{/each}
					</select>
				{/if}

				<!-- Type / Community -->
				<button
					onclick={toggleColorMode}
					class="px-3 py-2 glass rounded-xl text-xs transition whitespace-nowrap
						{colorMode === 'community' ? 'bg-synapse/20 text-synapse-glow border border-synapse/40' : 'text-dim hover:text-text'}"
				>
					{colorMode === 'type' ? '◉ Type' : '◎ Community'}
				</button>

				<!-- Freeze -->
				<button
					onclick={() => frozen = !frozen}
					class="px-3 py-2 glass rounded-xl text-xs transition whitespace-nowrap
						{frozen ? 'bg-blue-500/20 text-blue-300 border border-blue-500/40' : 'text-dim hover:text-text'}"
				>
					{frozen ? '▶ Resume' : '❚❚ Freeze'}
				</button>

				<!-- Unpin All -->
				<button
					onclick={() => graphComponent?.unpinAll()}
					class="px-3 py-2 glass rounded-xl text-xs text-dim hover:text-text transition whitespace-nowrap"
				>
					⊗ Unpin
				</button>

				<!-- Connection filter -->
				<div class="flex items-center gap-2 glass rounded-xl px-3 py-2">
					<span class="text-xs text-dim whitespace-nowrap">Min:</span>
					<input
						type="range" min="0" max="10" step="1"
						bind:value={minConnections}
						class="w-20 h-1.5 accent-synapse"
					/>
					<span class="text-xs text-synapse-glow w-4">{minConnections}</span>
				</div>

				<!-- Node count -->
				<select bind:value={maxNodes} onchange={() => loadGraph()}
					class="px-3 py-2 glass rounded-xl text-dim text-xs">
					<option value={50}>50</option>
					<option value={100}>100</option>
					<option value={150}>150</option>
					<option value={200}>200</option>
				</select>

				<!-- Dream -->
				<button
					onclick={triggerDream}
					disabled={isDreaming}
					class="px-4 py-2 rounded-xl bg-dream/20 border border-dream/40 text-dream-glow text-xs
						hover:bg-dream/30 transition-all backdrop-blur-sm disabled:opacity-50 whitespace-nowrap
						{isDreaming ? 'animate-pulse' : ''}"
				>
					{isDreaming ? '◈ Dreaming' : '◈ Dream'}
				</button>

				<!-- Reset View -->
				<button
					onclick={() => graphComponent?.resetView()}
					class="px-3 py-2 glass rounded-xl text-dim text-xs hover:text-text transition whitespace-nowrap"
				>
					⟲ Reset
				</button>
			</div>
		</div>

		<!-- Bottom info bar -->
		<div class="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-4 pointer-events-auto">
			<div class="glass rounded-xl px-5 py-2.5 text-sm text-dim flex items-center gap-4">
				{#if graphData}
					<span>{liveNodeCount} nodes</span>
					<span class="text-subtle">·</span>
					<span>{liveEdgeCount} edges</span>
				{/if}
				{#if infoMessage}
					<span class="text-subtle">·</span>
					<span class="text-synapse-glow">{infoMessage}</span>
				{/if}
			</div>
		</div>

		<!-- Edge type legend -->
		<div class="absolute bottom-4 left-4 glass rounded-xl px-4 py-3 pointer-events-auto">
			<div class="text-xs text-dim mb-1.5 font-medium">Edge Types</div>
			<div class="grid grid-cols-2 gap-x-4 gap-y-1">
				{#each [
					['semantic', 'rgba(139,92,246,0.6)'],
					['caused_by', 'rgba(251,191,36,0.8)'],
					['supersedes', 'rgba(239,68,68,0.8)'],
					['refines', 'rgba(16,185,129,0.7)'],
					['temporal', 'rgba(56,189,248,0.6)'],
					['cross_domain', 'rgba(168,85,247,0.6)'],
				] as [label, color]}
					<div class="flex items-center gap-2">
						<div class="w-4 h-1 rounded" style="background: {color}"></div>
						<span class="text-[10px] text-muted">{label}</span>
					</div>
				{/each}
			</div>
		</div>

		<!-- Selected memory panel -->
		{#if selectedMemory}
			<div class="absolute right-0 top-0 h-full w-80 max-w-[85vw] glass-panel p-5 overflow-y-auto
				border-l border-white/[0.06] pointer-events-auto">
				<div class="flex justify-between items-start mb-4">
					<h3 class="text-bright text-sm font-semibold">Memory Detail</h3>
					<button onclick={() => { selectedMemory = null; highlightIds = []; }} class="text-dim hover:text-text text-lg leading-none">×</button>
				</div>

				<div class="space-y-4">
					<div class="flex gap-2 flex-wrap">
						<span class="px-2 py-0.5 rounded-lg text-xs bg-synapse/20 text-synapse-glow">{selectedMemory.nodeType}</span>
						{#each selectedMemory.tags as tag}
							<span class="px-2 py-0.5 rounded-lg text-xs bg-white/[0.04] text-dim">{tag}</span>
						{/each}
					</div>

					<div class="text-sm text-text leading-relaxed whitespace-pre-wrap max-h-64 overflow-y-auto">{selectedMemory.content}</div>

					<!-- FSRS bars -->
					<div class="space-y-2">
						{#each [
							{ label: 'Retention', value: selectedMemory.retentionStrength },
							{ label: 'Storage', value: selectedMemory.storageStrength },
							{ label: 'Retrieval', value: selectedMemory.retrievalStrength }
						] as bar}
							<div>
								<div class="flex justify-between text-xs text-dim mb-0.5">
									<span>{bar.label}</span>
									<span>{(bar.value * 100).toFixed(1)}%</span>
								</div>
								<div class="h-1.5 bg-white/[0.04] rounded-full overflow-hidden">
									<div class="h-full rounded-full transition-all duration-500"
										style="width: {bar.value * 100}%; background: {
											bar.value > 0.7 ? '#10b981' :
											bar.value > 0.4 ? '#f59e0b' : '#ef4444'
										}"></div>
								</div>
							</div>
						{/each}
					</div>

					<RetentionCurve
						retention={selectedMemory.retentionStrength}
						stability={selectedMemory.storageStrength * 30}
					/>

					<div class="text-xs text-muted space-y-1">
						<div>Created: {new Date(selectedMemory.createdAt).toLocaleString()}</div>
						<div>Updated: {new Date(selectedMemory.updatedAt).toLocaleString()}</div>
						<div>Reviews: {selectedMemory.reviewCount ?? 0}</div>
					</div>

					<div class="flex gap-2 pt-2">
						<button
							onclick={handlePromote}
							disabled={promoteStatus === 'loading'}
							class="flex-1 px-3 py-2 rounded-xl text-xs transition
								{promoteStatus === 'success' ? 'bg-green-500/30 text-green-300 border border-green-500/50' :
								 promoteStatus === 'error' ? 'bg-red-500/30 text-red-300' :
								 promoteStatus === 'loading' ? 'bg-recall/10 text-recall/50 animate-pulse' :
								 'bg-recall/20 text-recall hover:bg-recall/30'}"
						>
							{promoteStatus === 'success' ? '✓ Promoted' :
							 promoteStatus === 'error' ? '✗ Failed' :
							 promoteStatus === 'loading' ? '...' :
							 '↑ Promote'}
						</button>
						<button
							onclick={handleDemote}
							disabled={demoteStatus === 'loading'}
							class="flex-1 px-3 py-2 rounded-xl text-xs transition
								{demoteStatus === 'success' ? 'bg-amber-500/30 text-amber-300 border border-amber-500/50' :
								 demoteStatus === 'error' ? 'bg-red-500/30 text-red-300' :
								 demoteStatus === 'loading' ? 'bg-decay/10 text-decay/50 animate-pulse' :
								 'bg-decay/20 text-decay hover:bg-decay/30'}"
						>
							{demoteStatus === 'success' ? '✓ Demoted' :
							 demoteStatus === 'error' ? '✗ Failed' :
							 demoteStatus === 'loading' ? '...' :
							 '↓ Demote'}
						</button>
					</div>

					<a href="{base}/explore"
						class="block text-center px-3 py-2 rounded-xl bg-dream/10 text-dream-glow text-xs hover:bg-dream/20 transition border border-dream/20"
					>
						◬ Explore Connections
					</a>
				</div>
			</div>
		{/if}

	</div><!-- close UI overlay -->
</div>
