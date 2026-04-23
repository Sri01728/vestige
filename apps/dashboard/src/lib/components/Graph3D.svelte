<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { browser } from '$app/environment';
	import type { GraphNode, GraphEdge } from '$types';
	import { NODE_TYPE_COLORS } from '$types';

	interface Props {
		nodes: GraphNode[];
		edges: GraphEdge[];
		centerId: string;
		isDreaming?: boolean;
		communityMap?: Map<string, number>;
		highlightIds?: string[];
		frozen?: boolean;
		minConnections?: number;
		onSelect?: (nodeId: string) => void;
		onInfoMessage?: (msg: string) => void;
	}

	let {
		nodes, edges, centerId, isDreaming = false, communityMap,
		highlightIds, frozen = false, minConnections = 0,
		onSelect, onInfoMessage,
	}: Props = $props();

	let container: HTMLDivElement;
	let graph: any = null;

	let revealedIds = new Set<string>();
	let focusNodeId: string | null = null;
	let pinnedIds = new Set<string>();

	const COMMUNITY_COLORS = [
		'#FF3CAC', '#00FFD1', '#FFB800', '#00A8FF',
		'#9D00FF', '#FF4757', '#00FF88', '#FF6B6B',
	];

	const EDGE_COLORS: Record<string, string> = {
		semantic:        'rgba(139,92,246,0.85)',
		caused_by:       'rgba(251,191,36,0.90)',
		supersedes:      'rgba(239,68,68,0.85)',
		contradicts:     'rgba(239,68,68,0.95)',
		refines:         'rgba(16,185,129,0.85)',
		shared_concepts: 'rgba(99,102,241,0.80)',
		shared_pattern:  'rgba(99,102,241,0.80)',
		cross_domain:    'rgba(168,85,247,0.85)',
		temporal:        'rgba(56,189,248,0.80)',
	};

	function getEdgeColor(type: string): string {
		return EDGE_COLORS[type] || EDGE_COLORS.semantic;
	}

	function getNodeColor(node: any): string {
		if (node._communityId !== undefined && node._communityId !== null) {
			return COMMUNITY_COLORS[node._communityId % COMMUNITY_COLORS.length];
		}
		return NODE_TYPE_COLORS[node.type] || '#8B95A5';
	}

	function stampCommunities() {
		if (!graph) return;
		const data = graph.graphData();
		const cm = communityMap;
		for (const node of data.nodes) {
			node._communityId = (cm && cm.size > 0) ? (cm.get(node.id) ?? null) : null;
		}
	}

	function getNeighborIds(nodeId: string): Set<string> {
		const neighbors = new Set<string>();
		neighbors.add(nodeId);
		for (const e of edges) {
			if (e.source === nodeId) neighbors.add(e.target);
			if (e.target === nodeId) neighbors.add(e.source);
		}
		return neighbors;
	}

	function connectionCount(nodeId: string): number {
		let count = 0;
		for (const e of edges) {
			if (e.source === nodeId || e.target === nodeId) count++;
		}
		return count;
	}

	function nodeVisible(node: any): boolean {
		if (minConnections > 0 && connectionCount(node.id) < minConnections) return false;
		if (revealedIds.size > 0 && !revealedIds.has(node.id)) return false;
		return true;
	}

	function linkVisible(link: any): boolean {
		const srcId = typeof link.source === 'object' ? link.source.id : link.source;
		const tgtId = typeof link.target === 'object' ? link.target.id : link.target;
		if (revealedIds.size > 0 && (!revealedIds.has(srcId) || !revealedIds.has(tgtId))) return false;
		if (minConnections > 0 && (connectionCount(srcId) < minConnections || connectionCount(tgtId) < minConnections)) return false;
		return true;
	}

	function refreshVisuals() {
		if (!graph) return;

		graph.nodeColor((node: any) => {
			if (!nodeVisible(node)) return 'rgba(0,0,0,0)';
			const hl = highlightIds;
			if (hl && hl.length > 0 && !hl.includes(node.id)) return '#1a1a2e';
			if (focusNodeId && !getNeighborIds(focusNodeId).has(node.id)) return '#1a1a2e';
			return getNodeColor(node);
		});

		graph.nodeVal((node: any) => {
			if (!nodeVisible(node)) return 0.001;
			return 3 + node.retention * 8;
		});

		graph.linkWidth((link: any) => {
			if (!linkVisible(link)) return 0;
			return 0.8 + link.weight * 2;
		});

		graph.linkColor((link: any) => {
			if (!linkVisible(link)) return 'rgba(0,0,0,0)';
			const hl = highlightIds;
			const srcId = typeof link.source === 'object' ? link.source.id : link.source;
			const tgtId = typeof link.target === 'object' ? link.target.id : link.target;
			if (hl && hl.length > 0 && !hl.includes(srcId) && !hl.includes(tgtId)) return 'rgba(139,92,246,0.03)';
			if (focusNodeId) {
				const fn = getNeighborIds(focusNodeId);
				if (!fn.has(srcId) || !fn.has(tgtId)) return 'rgba(139,92,246,0.03)';
			}
			return getEdgeColor(link.type);
		});

		graph.linkDirectionalParticles((link: any) => {
			if (!linkVisible(link)) return 0;
			return Math.ceil(link.weight * 3);
		});

		// Force 3d-force-graph to re-evaluate all accessors
		graph.graphData(graph.graphData());
	}

	export function zoomToNode(nodeId: string) {
		if (!graph) return;
		const data = graph.graphData();
		const node = data.nodes.find((n: any) => n.id === nodeId);
		if (!node) return;
		const distance = 60;
		const distRatio = 1 + distance / Math.hypot(node.x || 1, node.y || 1, node.z || 1);
		graph.cameraPosition(
			{ x: node.x * distRatio, y: node.y * distRatio, z: node.z * distRatio },
			node,
			800
		);
	}

	export function resetView() {
		if (!graph) return;
		revealedIds = new Set();
		focusNodeId = null;
		refreshVisuals();
		graph.zoomToFit(400, 40);
		onInfoMessage?.('View reset');
	}

	export function doFreeze(freeze: boolean) {
		if (!graph) return;
		if (freeze) {
			graph.pauseAnimation();
			onInfoMessage?.('Physics frozen');
		} else {
			graph.resumeAnimation();
			graph.d3ReheatSimulation();
			onInfoMessage?.('Physics resumed');
		}
	}

	export function unpinAll() {
		if (!graph) return;
		const data = graph.graphData();
		data.nodes.forEach((n: any) => { n.fx = undefined; n.fy = undefined; n.fz = undefined; });
		pinnedIds = new Set();
		graph.d3ReheatSimulation();
		onInfoMessage?.('All nodes unpinned');
	}

	onMount(async () => {
		if (!browser) return;

		const module = await import('3d-force-graph');
		const ForceGraph3D = module.default as any;

		const gData = {
			nodes: nodes.map(n => ({ ...n, _size: 3 + n.retention * 8 })),
			links: edges.map(e => ({ source: e.source, target: e.target, weight: e.weight, type: e.type })),
		};

		graph = ForceGraph3D()(container)
			.graphData(gData)
			.backgroundColor('#05050f')
			.showNavInfo(false);

		// Node config
		graph
			.nodeVal((node: any) => node._size)
			.nodeColor((node: any) => getNodeColor(node))
			.nodeOpacity(0.9)
			.nodeResolution(12)
			.nodeLabel((node: any) => {
				const ret = (node.retention * 100).toFixed(0);
				const conns = connectionCount(node.id);
				const pinLabel = pinnedIds.has(node.id) ? ' · 📌 pinned' : '';
				return `<div style="background:rgba(0,0,0,0.85);padding:8px 12px;border-radius:8px;color:#e2e8f0;font-size:12px;max-width:320px;border:1px solid rgba(139,92,246,0.2);">
					<div style="font-weight:600;margin-bottom:3px;">${node.label}</div>
					<div style="color:#8b95a5;font-size:10px;">${node.type} · ${ret}% · ${conns} conn${pinLabel}</div>
					<div style="display:flex;gap:4px;margin-top:5px;flex-wrap:wrap;">
						${(node.tags || []).slice(0, 6).map((t: string) => `<span style="background:rgba(139,92,246,0.3);padding:1px 6px;border-radius:4px;font-size:9px;color:#a78bfa;">${t}</span>`).join('')}
					</div>
				</div>`;
			});

		// Edge config (NO linkLineDash — doesn't exist in 3D version)
		graph
			.linkWidth((link: any) => 0.8 + link.weight * 2)
			.linkColor((link: any) => getEdgeColor(link.type))
			.linkOpacity(0.7)
			.linkDirectionalParticles((link: any) => Math.ceil(link.weight * 3))
			.linkDirectionalParticleWidth(1.5)
			.linkDirectionalParticleSpeed(0.006)
			.linkDirectionalParticleColor(() => '#a78bfa')
			.linkLabel((link: any) => `<span style="background:rgba(0,0,0,0.7);padding:2px 6px;border-radius:4px;color:#a78bfa;font-size:10px;">${link.type}</span>`);

		// Physics
		graph
			.d3AlphaDecay(0.02)
			.d3VelocityDecay(0.3)
			.warmupTicks(100)
			.cooldownTicks(200);

		// Interactions
		graph.onNodeClick((node: any) => {
			if (onSelect) onSelect(node.id);

			const neighbors = getNeighborIds(node.id);
			if (revealedIds.size === 0) {
				revealedIds = neighbors;
				focusNodeId = node.id;
				onInfoMessage?.(`Showing ${neighbors.size} connected nodes`);
			} else if (focusNodeId === node.id) {
				revealedIds = new Set();
				focusNodeId = null;
				onInfoMessage?.('View reset');
			} else {
				const merged = new Set(revealedIds);
				neighbors.forEach(id => merged.add(id));
				revealedIds = merged;
				focusNodeId = node.id;
				onInfoMessage?.(`Expanded to ${merged.size} nodes`);
			}
			refreshVisuals();

			const distance = 60;
			const distRatio = 1 + distance / Math.hypot(node.x, node.y, node.z);
			graph.cameraPosition(
				{ x: node.x * distRatio, y: node.y * distRatio, z: node.z * distRatio },
				node,
				800
			);
		});

		graph.onNodeDragEnd((node: any) => {
			node.fx = node.x;
			node.fy = node.y;
			node.fz = node.z;
			pinnedIds.add(node.id);
			onInfoMessage?.('Node pinned');
		});

		graph.onBackgroundClick(() => {
			revealedIds = new Set();
			focusNodeId = null;
			refreshVisuals();
			onInfoMessage?.('');
			if (onSelect) onSelect('');
		});

		// Forces
		graph.d3Force('charge')?.strength(-120);
		graph.d3Force('link')?.distance((link: any) => 30 + (1 - link.weight) * 50);

		// Fit after warmup
		setTimeout(() => graph?.zoomToFit(400, 40), 500);
	});

	onDestroy(() => {
		if (graph) {
			graph._destructor?.();
			graph = null;
		}
	});

	$effect(() => {
		if (!graph) return;
		const _ids = highlightIds;
		refreshVisuals();
	});

	$effect(() => {
		if (!graph) return;
		const _map = communityMap;
		stampCommunities();
		refreshVisuals();
	});

	$effect(() => {
		if (!graph) return;
		const _mc = minConnections;
		refreshVisuals();
	});

	$effect(() => {
		if (!graph) return;
		doFreeze(frozen);
	});

	$effect(() => {
		if (!graph) return;
		if (isDreaming) {
			graph.linkDirectionalParticleSpeed(0.015);
			graph.linkDirectionalParticleWidth(2.5);
		} else {
			graph.linkDirectionalParticleSpeed(0.006);
			graph.linkDirectionalParticleWidth(1.5);
		}
	});
</script>

<div bind:this={container} class="w-full h-full"></div>
