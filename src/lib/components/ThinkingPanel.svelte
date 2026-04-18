<script lang="ts">
  import { onAiToken, aiCancel } from "$lib/bridge";
  import { onMount, onDestroy } from "svelte";
  import { Card, CardHeader, CardTitle, CardContent, CardFooter } from "$lib/components/ui/card";
  import { Button } from "$lib/components/ui/button";
  import { fade, fly } from "svelte/transition";

  let { active = false } = $props();

  let tokens = $state("");
  let unlisten: (() => void) | undefined;

  onMount(async () => {
    unlisten = await onAiToken((token) => {
      tokens += token;
    });
  });

  onDestroy(() => {
    unlisten?.();
  });

  function clear() {
    tokens = "";
  }

  async function handleCancel() {
    await aiCancel();
  }

  $effect(() => {
    if (!active) {
      // maybe clear?
    }
  });
</script>

{#if active || tokens}
  <div
    transition:fly={{ y: 20, duration: 300 }}
    class="fixed bottom-20 right-4 w-96 max-h-[70vh] z-50 shadow-2xl"
  >
    <Card class="flex flex-col h-full border-primary/20 bg-background/95 backdrop-blur">
      <CardHeader class="py-3">
        <div class="flex items-center justify-between">
          <CardTitle class="text-sm font-medium">Gemma thinking...</CardTitle>
          <Button variant="ghost" size="sm" onclick={clear} disabled={!tokens}>Clear</Button>
        </div>
      </CardHeader>
      <CardContent class="flex-1 overflow-hidden p-0">
        <div class="h-64 p-4 overflow-auto">
          <div class="whitespace-pre-wrap text-sm font-mono">
            {tokens}
          </div>
        </div>
      </CardContent>
      <CardFooter class="py-3 flex justify-end gap-2">
        <Button variant="outline" size="sm" onclick={handleCancel}>Cancel</Button>
      </CardFooter>
    </Card>
  </div>
{/if}
