import type { GraphNode, GraphEdge } from '$types';

export async function detectCommunities(
	nodes: GraphNode[],
	edges: GraphEdge[]
): Promise<Map<string, number>> {
	try {
		const { default: Graph } = await import('graphology');
		const { default: louvain } = await import('graphology-communities-louvain');

		const graph = new Graph({ type: 'undirected', multi: false });

		for (const node of nodes) {
			if (!graph.hasNode(node.id)) {
				graph.addNode(node.id);
			}
		}

		for (const edge of edges) {
			if (graph.hasNode(edge.source) && graph.hasNode(edge.target)) {
				if (!graph.hasEdge(edge.source, edge.target)) {
					graph.addEdge(edge.source, edge.target, { weight: edge.weight });
				}
			}
		}

		const communities = louvain(graph, { resolution: 1 });

		const result = new Map<string, number>();
		for (const [nodeId, communityId] of Object.entries(communities)) {
			result.set(nodeId, communityId as number);
		}

		return result;
	} catch {
		return new Map();
	}
}
