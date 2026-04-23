<script lang="ts">
	import { browser } from '$app/environment';

	interface HeatmapEntry {
		date: string;
		count: number;
		types: string[];
	}

	interface Props {
		data: HeatmapEntry[];
		onselect?: (date: string) => void;
	}

	let { data, onselect }: Props = $props();

	let dataMap = $derived(new Map(data.map((d) => [d.date, d])));

	interface Cell { date: string; count: number; types: string[]; row: number; col: number; }

	let grid = $derived.by((): Cell[] => {
		const today = new Date();
		const cells: Cell[] = [];
		const year = today.getFullYear();
		const startDate = new Date(year, 0, 1);
		const endDate = new Date(year, 11, 31);

		// Start from Jan 1, pad to align Sunday = row 0
		const startDow = startDate.getDay(); // 0=Sun
		const gridStart = new Date(startDate);
		gridStart.setDate(gridStart.getDate() - startDow);

		const cursor = new Date(gridStart);
		let col = 0;
		while (cursor <= endDate) {
			for (let row = 0; row < 7; row++) {
				const d = new Date(cursor);
				d.setDate(d.getDate() + row);
				const dateStr = `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
				const entry = dataMap.get(dateStr);
				const inRange = d >= startDate && d <= endDate;
				cells.push({
					date: dateStr,
					count: inRange ? (entry?.count ?? 0) : -1, // -1 = outside range
					types: entry?.types ?? [],
					row, col,
				});
			}
			cursor.setDate(cursor.getDate() + 7);
			col++;
		}
		return cells;
	});

	let totalCols = $derived(Math.max(...grid.map((c) => c.col), 0) + 1);

	let monthLabels = $derived.by(() => {
		const labels: { label: string; col: number }[] = [];
		const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
		let lastMonth = -1;
		for (const cell of grid) {
			if (cell.row !== 0) continue;
			const month = new Date(cell.date).getMonth();
			const cellYear = new Date(cell.date).getFullYear();
			if (month !== lastMonth && cellYear === new Date().getFullYear()) {
				labels.push({ label: months[month], col: cell.col });
				lastMonth = month;
			}
		}
		return labels;
	});

	let totalMemories = $derived(data.reduce((s, d) => s + d.count, 0));
	let isFuture = (dateStr: string) => new Date(dateStr) > new Date();

	function color(count: number): string {
		if (count < 0) return 'transparent';
		if (count === 0) return 'rgba(139,92,246,0.08)';
		if (count <= 2) return 'rgba(99,102,241,0.45)';
		if (count <= 5) return 'rgba(99,102,241,0.7)';
		return 'rgba(129,140,248,0.95)';
	}

	let tip = $state({ show: false, x: 0, y: 0, date: '', count: 0, types: [] as string[] });

	function showTip(e: MouseEvent, cell: Cell) {
		if (!browser || cell.count < 0) return;
		const el = e.currentTarget as HTMLElement;
		const wrap = el.closest('.hm-wrap') as HTMLElement;
		if (!wrap) return;
		const r = el.getBoundingClientRect();
		const wr = wrap.getBoundingClientRect();
		tip = { show: true, x: r.left - wr.left + r.width/2, y: r.top - wr.top - 8, date: cell.date, count: cell.count, types: cell.types };
	}
</script>

<div class="glass-subtle rounded-xl p-5">
	<!-- Header -->
	<div class="flex items-center justify-between mb-4">
		<div>
			<h3 class="text-sm text-bright font-medium">{totalMemories} memories in {new Date().getFullYear()}</h3>
		</div>
		<div class="flex items-center gap-1 text-[10px] text-muted">
			<span>Less</span>
			{#each [0, 1, 3, 6] as n}
				<div class="w-[10px] h-[10px] rounded-[2px]" style="background:{color(n)}"></div>
			{/each}
			<span>More</span>
		</div>
	</div>

	<!-- Heatmap -->
	<div class="hm-wrap relative overflow-x-auto">
		<!-- Month labels row -->
		<div class="flex ml-[30px] mb-1">
			{#each Array(totalCols) as _, i}
				{@const lbl = monthLabels.find(m => m.col === i)}
				<div class="text-[10px] text-muted" style="width:14px;min-width:14px;margin-right:2px;">
					{lbl?.label ?? ''}
				</div>
			{/each}
		</div>

		<div class="flex">
			<!-- Day labels -->
			<div class="flex flex-col justify-around shrink-0 pr-1" style="width:30px;height:{7*14 + 6*2}px;">
				{#each ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'] as day, i}
					{#if i % 2 === 1}
						<span class="text-[9px] text-muted leading-none">{day}</span>
					{:else}
						<span class="text-[9px] leading-none">&nbsp;</span>
					{/if}
				{/each}
			</div>

			<!-- Cell grid -->
			<div style="display:grid;grid-template-rows:repeat(7,14px);grid-auto-columns:14px;grid-auto-flow:column;gap:2px;">
				{#each grid as cell (cell.date + cell.row)}
					{#if cell.count < 0}
						<div style="grid-row:{cell.row+1};grid-column:{cell.col+1};width:14px;height:14px;"></div>
					{:else}
						<button
							style="grid-row:{cell.row+1};grid-column:{cell.col+1};width:14px;height:14px;background:{isFuture(cell.date) ? 'transparent' : color(cell.count)};border-radius:3px;border:none;padding:0;cursor:pointer;transition:transform 0.1s;"
							onmouseenter={(e) => showTip(e, cell)}
							onmouseleave={() => tip = {...tip, show:false}}
							onclick={() => onselect?.(cell.date)}
							class="hover:outline hover:outline-1 hover:outline-white/30 hover:scale-110 focus:outline-none"
							aria-label="{cell.date}: {cell.count} memories"
						></button>
					{/if}
				{/each}
			</div>
		</div>

		<!-- Tooltip -->
		{#if tip.show}
			<div
				class="absolute pointer-events-none glass-panel rounded-lg px-3 py-2 text-xs -translate-x-1/2 -translate-y-full"
				style="left:{tip.x}px;top:{tip.y}px;z-index:50;"
			>
				<div class="text-bright font-medium">{tip.date}</div>
				<div class="text-dim">{tip.count} {tip.count === 1 ? 'memory' : 'memories'}</div>
				{#if tip.types.length > 0}
					<div class="text-muted mt-0.5">{tip.types.slice(0,5).join(', ')}</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
