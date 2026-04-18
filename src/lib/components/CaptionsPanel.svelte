<script lang="ts">
  let { captions }: { captions: string[] } = $props();
</script>

<div class="flex flex-col gap-1 h-full overflow-y-auto p-2">
  {#if captions.length === 0}
    <p class="text-xs text-muted-foreground italic">No captions yet…</p>
  {:else}
    {#each captions as caption, i (i)}
      <p class="text-sm bg-muted rounded px-2 py-1">{caption}</p>
    {/each}
  {/if}
</div>
