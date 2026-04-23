<script lang="ts">
	import { goto } from '$app/navigation';
	import { base } from '$app/paths';
	import { onMount } from 'svelte';
	onMount(() => goto(`${base}/analytics`, { replaceState: true }));
</script>

<div class="p-6 text-center text-dim">Redirecting to Command Center...</div>
