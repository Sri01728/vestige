<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$stores/api';
	import type { IntentionItem } from '$types';

	let intentions: IntentionItem[] = $state([]);
	let loading = $state(true);
	let statusFilter = $state('active');

	// Create form
	let showForm = $state(false);
	let newContent = $state('');
	let newTriggerType = $state('context');
	let newPriority = $state(2);
	let newDeadline = $state('');
	let creating = $state(false);

	// Action feedback
	let actionFeedback: Record<string, string> = $state({});

	const STATUS_STYLES: Record<string, { bg: string; text: string; label: string }> = {
		active: { bg: 'bg-synapse/10 border-synapse/30', text: 'text-synapse-glow', label: 'Active' },
		fulfilled: { bg: 'bg-green-500/10 border-green-500/30', text: 'text-green-400', label: 'Done' },
		cancelled: { bg: 'bg-white/[0.03] border-white/[0.06]', text: 'text-dim', label: 'Cancelled' },
		snoozed: { bg: 'bg-dream/10 border-dream/30', text: 'text-dream-glow', label: 'Snoozed' },
	};

	const PRIORITY_LABELS: Record<number, { label: string; color: string }> = {
		1: { label: 'Low', color: 'text-muted' },
		2: { label: 'Normal', color: 'text-dim' },
		3: { label: 'High', color: 'text-amber-400' },
		4: { label: 'Critical', color: 'text-red-400' },
	};

	const TRIGGER_ICONS: Record<string, string> = {
		time: '⏰', context: '◎', event: '⚡', duration: '⏱', recurring: '🔁',
	};

	onMount(() => loadData());

	async function loadData() {
		loading = true;
		try {
			const res = await api.intentions(statusFilter);
			intentions = res.intentions || [];
		} catch { intentions = []; }
		finally { loading = false; }
	}

	async function changeFilter(status: string) {
		statusFilter = status;
		await loadData();
	}

	async function createIntention() {
		if (!newContent.trim()) return;
		creating = true;
		try {
			await api.createIntention(newContent, {
				trigger_type: newTriggerType,
				priority: newPriority,
				deadline: newDeadline || undefined,
			});
			newContent = '';
			newDeadline = '';
			newPriority = 2;
			showForm = false;
			await loadData();
		} catch { /* failed */ }
		finally { creating = false; }
	}

	async function updateStatus(id: string, status: string) {
		actionFeedback[id] = 'loading';
		try {
			await api.updateIntention(id, status);
			actionFeedback[id] = status;
			setTimeout(() => { delete actionFeedback[id]; actionFeedback = actionFeedback; }, 1500);
			await loadData();
		} catch {
			actionFeedback[id] = 'error';
			setTimeout(() => { delete actionFeedback[id]; actionFeedback = actionFeedback; }, 2000);
		}
	}

	function formatDate(d: string | undefined): string {
		if (!d) return '';
		try { return new Date(d).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }); }
		catch { return d; }
	}

	function isOverdue(deadline: string | undefined): boolean {
		if (!deadline) return false;
		return new Date(deadline) < new Date();
	}
</script>

<div class="p-6 max-w-4xl mx-auto space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-xl text-bright font-semibold">Intentions</h1>
		<button
			onclick={() => showForm = !showForm}
			class="px-4 py-2 rounded-xl text-sm transition
				{showForm ? 'bg-white/[0.06] text-dim' : 'bg-synapse/20 border border-synapse/40 text-synapse-glow hover:bg-synapse/30'}"
		>
			{showForm ? '✕ Cancel' : '+ New Intention'}
		</button>
	</div>

	<!-- Create form -->
	{#if showForm}
		<div class="glass rounded-xl p-5 space-y-4" style="border-color: rgba(139,92,246,0.2)">
			<div>
				<label class="text-xs text-dim block mb-1">What do you want to remember to do?</label>
				<textarea
					bind:value={newContent}
					placeholder="e.g., Check the Redis connection pool settings before next deploy..."
					rows="2"
					class="w-full px-3 py-2 glass rounded-lg text-sm text-text placeholder:text-muted focus:outline-none focus:border-synapse/40 resize-none"
				></textarea>
			</div>

			<div class="grid grid-cols-3 gap-3">
				<div>
					<label class="text-xs text-dim block mb-1">Trigger</label>
					<select bind:value={newTriggerType} class="w-full px-3 py-2 glass rounded-lg text-xs text-dim">
						<option value="context">◎ Context (when topic matches)</option>
						<option value="time">⏰ Time (at deadline)</option>
						<option value="event">⚡ Event (on activity)</option>
					</select>
				</div>
				<div>
					<label class="text-xs text-dim block mb-1">Priority</label>
					<select bind:value={newPriority} class="w-full px-3 py-2 glass rounded-lg text-xs text-dim">
						<option value={1}>Low</option>
						<option value={2}>Normal</option>
						<option value={3}>High</option>
						<option value={4}>Critical</option>
					</select>
				</div>
				<div>
					<label class="text-xs text-dim block mb-1">Deadline (optional)</label>
					<input
						type="datetime-local"
						bind:value={newDeadline}
						class="w-full px-3 py-2 glass rounded-lg text-xs text-dim"
					/>
				</div>
			</div>

			<button
				onclick={createIntention}
				disabled={creating || !newContent.trim()}
				class="px-4 py-2 rounded-xl text-sm bg-synapse/20 border border-synapse/40 text-synapse-glow hover:bg-synapse/30 transition disabled:opacity-50"
			>
				{creating ? 'Creating...' : 'Create Intention'}
			</button>
		</div>
	{/if}

	<!-- Status filter tabs -->
	<div class="flex gap-1.5">
		{#each ['active', 'fulfilled', 'snoozed', 'cancelled', 'all'] as status}
			<button
				onclick={() => changeFilter(status)}
				class="px-3 py-1.5 rounded-lg text-xs transition
					{statusFilter === status ? 'bg-synapse/20 text-synapse-glow border border-synapse/40' : 'glass-subtle text-dim hover:bg-white/[0.03]'}"
			>
				{status.charAt(0).toUpperCase() + status.slice(1)}
			</button>
		{/each}
	</div>

	<!-- Intentions list -->
	{#if loading}
		<div class="space-y-2">
			{#each Array(4) as _}
				<div class="h-20 glass-subtle rounded-xl animate-pulse"></div>
			{/each}
		</div>
	{:else if intentions.length === 0}
		<div class="text-center py-16 text-dim">
			<div class="text-4xl mb-3 opacity-20">◇</div>
			<p class="text-sm">No {statusFilter === 'all' ? '' : statusFilter + ' '}intentions.</p>
			<p class="text-xs text-muted mt-2">Create one above, or say "Remind me to..." in Claude Code.</p>
		</div>
	{:else}
		<div class="space-y-2">
			{#each intentions as intention (intention.id)}
				{@const style = STATUS_STYLES[intention.status] || STATUS_STYLES.active}
				{@const priority = PRIORITY_LABELS[intention.priority] || PRIORITY_LABELS[2]}
				{@const overdue = intention.status === 'active' && isOverdue(intention.deadline)}
				{@const feedback = actionFeedback[intention.id]}

				<div class="p-4 glass-subtle rounded-xl border {overdue ? '!border-red-500/40' : style.bg} transition-all
					{feedback === 'fulfilled' ? '!bg-green-500/10' : feedback === 'cancelled' ? '!bg-white/[0.02]' : ''}">
					<div class="flex items-start gap-3">
						<!-- Trigger icon -->
						<div class="w-9 h-9 rounded-lg bg-white/[0.04] flex items-center justify-center text-lg flex-shrink-0">
							{TRIGGER_ICONS[intention.trigger_type] || '◇'}
						</div>

						<div class="flex-1 min-w-0">
							<p class="text-sm text-text leading-relaxed">{intention.content}</p>

							<div class="flex flex-wrap items-center gap-2 mt-2">
								<!-- Status badge -->
								<span class="px-2 py-0.5 text-[10px] rounded-lg border {style.bg} {style.text}">
									{style.label}
								</span>
								<!-- Priority -->
								<span class="text-[10px] {priority.color}">
									{priority.label}
								</span>
								<!-- Deadline -->
								{#if intention.deadline}
									<span class="text-[10px] {overdue ? 'text-red-400 font-semibold' : 'text-dream-glow'}">
										{overdue ? '⚠ OVERDUE' : ''} due {formatDate(intention.deadline)}
									</span>
								{/if}
								<!-- Created -->
								<span class="text-[10px] text-muted">
									created {formatDate(intention.created_at)}
								</span>
							</div>
						</div>

						<!-- Action buttons (only for active intentions) -->
						{#if intention.status === 'active'}
							<div class="flex gap-1.5 flex-shrink-0">
								{#if feedback === 'loading'}
									<div class="px-3 py-1.5 text-[10px] text-dim animate-pulse">...</div>
								{:else if feedback}
									<div class="px-3 py-1.5 text-[10px] text-green-400">✓</div>
								{:else}
									<button
										onclick={() => updateStatus(intention.id, 'fulfilled')}
										class="px-2.5 py-1.5 rounded-lg text-[10px] bg-green-500/10 text-green-400 hover:bg-green-500/20 transition border border-green-500/20"
									>✓ Done</button>
									<button
										onclick={() => updateStatus(intention.id, 'snoozed')}
										class="px-2.5 py-1.5 rounded-lg text-[10px] bg-dream/10 text-dream-glow hover:bg-dream/20 transition border border-dream/20"
									>💤 Snooze</button>
									<button
										onclick={() => updateStatus(intention.id, 'cancelled')}
										class="px-2.5 py-1.5 rounded-lg text-[10px] bg-white/[0.03] text-dim hover:bg-white/[0.06] transition border border-white/[0.06]"
									>✕</button>
								{/if}
							</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
