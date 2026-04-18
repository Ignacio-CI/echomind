<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "$lib/components/ui/card";
  import { callPing, onTranscriptFinal, onTranscriptPartial } from "$lib/bridge.js";
  import CaptionsPanel from "$lib/components/CaptionsPanel.svelte";

  let pingResult = $state("");
  let captions = $state<string[]>([]);

  let unlistenPartial: (() => void) | undefined;
  let unlistenFinal: (() => void) | undefined;

  onMount(async () => {
    try {
      pingResult = await callPing();
    } catch {
      pingResult = "error";
    }

    unlistenPartial = await onTranscriptPartial((text) => {
      captions.push(text);
    });

    unlistenFinal = await onTranscriptFinal((text) => {
      captions.push(text);
    });
  });

  onDestroy(() => {
    unlistenPartial?.();
    unlistenFinal?.();
  });
</script>

<div class="grid grid-cols-2 gap-4 h-full">
  <div class="flex flex-col items-center justify-center">
    <Card>
      <CardHeader>
        <CardTitle>Welcome to EchoMind</CardTitle>
        <CardDescription>Start recording to begin your AI-powered meeting analysis.</CardDescription>
      </CardHeader>
      <CardContent></CardContent>
    </Card>
    <p class="text-xs text-muted-foreground text-center mt-2">Bridge: {pingResult}</p>
  </div>

  <div class="flex flex-col">
    <Card class="h-full flex flex-col">
      <CardHeader>
        <CardTitle>Live Captions</CardTitle>
      </CardHeader>
      <CardContent class="flex-1 overflow-hidden">
        <CaptionsPanel {captions} />
      </CardContent>
    </Card>
  </div>
</div>
