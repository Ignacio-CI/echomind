<script lang="ts">
  import '../app.css';
  import { ModeWatcher } from 'mode-watcher';
  import { Toaster } from '$lib/components/ui/sonner';
  import { Button } from '$lib/components/ui/button';
  import { Mic, Square, Globe } from 'lucide-svelte';
  import { t, setLocale, getLocale } from '$lib/i18n/index.svelte.js';
  import { audioStart, audioStop, onAudioLevel } from '$lib/bridge.js';
  import LevelMeter from '$lib/components/LevelMeter.svelte';

  let { children } = $props();

  let isRecording = $state(false);
  let audioLevel = $state(0);
  let unlistenLevel: (() => void) | undefined = $state(undefined);
</script>

<ModeWatcher />
<Toaster />

<div class="flex h-screen flex-col">
  <header class="bg-card border-b flex items-center justify-between px-4 py-2">
    <span class="font-bold text-lg">{t("appTitle")}</span>
    <div class="flex items-center gap-2">
      {#if isRecording}
        <LevelMeter level={audioLevel} />
      {/if}
      <Button
        variant={isRecording ? "destructive" : "default"}
        onclick={async () => {
          isRecording = !isRecording;
          if (isRecording) {
            await audioStart();
            unlistenLevel = await onAudioLevel(lvl => { audioLevel = lvl; });
          } else {
            await audioStop();
            unlistenLevel?.();
            audioLevel = 0;
          }
        }}
      >
        {#if isRecording}
          <Square class="mr-2 h-4 w-4" />
          {t("stopRecording")}
        {:else}
          <Mic class="mr-2 h-4 w-4" />
          {t("startRecording")}
        {/if}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        title={t("language")}
        onclick={() => setLocale(getLocale() === "en" ? "es" : "en")}
      >
        <Globe class="h-4 w-4" />
      </Button>
    </div>
  </header>

  <main class="flex-1 overflow-auto p-4">
    {@render children()}
  </main>

  <footer class="border-t px-4 py-2 text-xs text-muted-foreground">
    {isRecording ? t("statusRecording") : t("statusReady")}
  </footer>
</div>
