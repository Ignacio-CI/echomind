<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "$lib/components/ui/card";
  import { Button } from "$lib/components/ui/button";
  import { Tabs, TabsContent, TabsList, TabsTrigger } from "$lib/components/ui/tabs";
  import {
    callPing, onTranscriptFinal, onTranscriptPartial, aiGenerate,
    onSummaryUpdate, meetingSummarize, audioStart, audioStop,
    onAudioLevel, meetingCreate, meetingGetTranscript,
    meetingGetSummary, onAiStatus, modelsPrepare, modelsStatus, type Meeting
  } from "$lib/bridge.js";
  import { t, setLocale, getLocale } from '$lib/i18n/index.svelte.js';
  import { Mic, Square, Globe, History, Activity } from 'lucide-svelte';
  import { toast } from "svelte-sonner";
  
  import CaptionsPanel from "$lib/components/CaptionsPanel.svelte";
  import ThinkingPanel from "$lib/components/ThinkingPanel.svelte";
  import LevelMeter from '$lib/components/LevelMeter.svelte';
  import MeetingHistory from "$lib/components/MeetingHistory.svelte";

  let pingResult = $state("");
  let captions = $state<string[]>([]);
  let summary = $state("");
  let isThinking = $state(false);
  let isRecording = $state(false);
  let isStarting = $state(false);
  let isLoadingModels = $state(false);
  let audioLevel = $state(0);
  let currentTab = $state("live");

  let unlistenPartial: (() => void) | undefined;
  let unlistenFinal: (() => void) | undefined;
  let unlistenSummary: (() => void) | undefined;
  let unlistenLevel: (() => void) | undefined;
  let unlistenStatus: (() => void) | undefined;

  async function toggleRecording() {
    const nextState = !isRecording;
    try {
      if (nextState) {
        isStarting = true;
        const alreadyLoaded = await modelsStatus();
        if (!alreadyLoaded) {
          isLoadingModels = true;
          await modelsPrepare();
          isLoadingModels = false;
        }
        const id = crypto.randomUUID();
        const locale = getLocale();
        await meetingCreate(id, `Meeting ${new Date().toLocaleString()}`, locale);
        await audioStart();
        unlistenLevel = await onAudioLevel(lvl => { audioLevel = lvl; });
        captions = [];
        summary = "";
        currentTab = "live";
        isRecording = true;
      } else {
        await audioStop();
        unlistenLevel?.();
        audioLevel = 0;
        isRecording = false;
      }
    } catch (e: any) {
      console.error(e);
      let errorMessage = "Unknown error";
      if (typeof e === 'string') {
        errorMessage = e;
      } else if (e && typeof e === 'object') {
        errorMessage = e.message || e.error || JSON.stringify(e);
      }
      toast.error(`Failed to ${nextState ? 'start' : 'stop'} recording: ${errorMessage}`);
    } finally {
      isStarting = false;
      isLoadingModels = false;
    }
  }

  async function handleSelectMeeting(meeting: Meeting) {
    currentTab = "live";
    isThinking = true;
    try {
      const transcript = await meetingGetTranscript(meeting.id);
      captions = transcript.map(t => t.text);
      const savedSummary = await meetingGetSummary(meeting.id);
      summary = savedSummary || "";
    } catch (e) {
      console.error(e);
    } finally {
      isThinking = false;
    }
  }

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

    unlistenSummary = await onSummaryUpdate((text) => {
      summary = text;
    });

    unlistenStatus = await onAiStatus((status) => {
      toast(status);
    });
  });

  onDestroy(() => {
    unlistenPartial?.();
    unlistenFinal?.();
    unlistenSummary?.();
    unlistenLevel?.();
    unlistenStatus?.();
  });
</script>

<header class="bg-card border-b flex items-center justify-between px-4 py-2 shrink-0">
  <div class="flex items-center gap-4">
    <span class="font-bold text-lg">{t("appTitle")}</span>
    <Tabs bind:value={currentTab} class="w-[200px]">
      <TabsList class="grid w-full grid-cols-2">
        <TabsTrigger value="live" class="flex items-center gap-1">
          <Activity class="h-3.5 w-3.5" />
          Live
        </TabsTrigger>
        <TabsTrigger value="history" class="flex items-center gap-1">
          <History class="h-3.5 w-3.5" />
          History
        </TabsTrigger>
      </TabsList>
    </Tabs>
  </div>

  <div class="flex items-center gap-2">
    {#if isRecording}
      <LevelMeter level={audioLevel} />
    {/if}
    <Button
      variant={isRecording ? "destructive" : "default"}
      onclick={toggleRecording}
      disabled={isStarting}
    >
      {#if isLoadingModels}
        <span class="animate-pulse">Loading models…</span>
      {:else if isStarting}
        <span class="animate-pulse">{t("starting")}</span>
      {:else if isRecording}
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
      onclick={() => setLocale(getLocale() === "en" ? "es" : "en")}
    >
      <Globe class="h-4 w-4" />
    </Button>
  </div>
</header>

<main class="flex-1 overflow-hidden relative">
  <div class="h-full w-full p-4 overflow-auto">
    {#if currentTab === "live"}
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4 h-full min-h-0">
        <!-- Left Pane -->
        <div class="flex flex-col gap-4 min-h-0">
          <Card class="shrink-0">
            <CardHeader class="py-3 px-4">
              <CardTitle class="text-sm font-medium">Controls</CardTitle>
            </CardHeader>
            <CardContent class="py-0 px-4 pb-3 flex gap-2">
              <Button onclick={() => meetingSummarize(getLocale())} variant="secondary" size="sm" disabled={isRecording && captions.length === 0}>
                Manual Summary
              </Button>
              <Button onclick={() => { captions = []; summary = ""; }} variant="outline" size="sm">
                Clear View
              </Button>
            </CardContent>
          </Card>

          <Card class="flex-1 flex flex-col min-h-0">
            <CardHeader class="py-3 px-4 shrink-0">
              <CardTitle class="text-sm font-medium">Live Captions</CardTitle>
            </CardHeader>
            <CardContent class="flex-1 overflow-auto p-4 pt-0">
              <CaptionsPanel {captions} />
            </CardContent>
          </Card>
        </div>

        <!-- Right Pane -->
        <div class="flex flex-col min-h-0">
          <Card class="h-full flex flex-col border-primary/20 min-h-0">
            <CardHeader class="py-3 px-4 shrink-0">
              <CardTitle class="text-sm font-medium">Analysis & Summary</CardTitle>
            </CardHeader>
            <CardContent class="flex-1 overflow-auto p-4 pt-0">
              <div class="whitespace-pre-wrap text-sm leading-relaxed prose prose-sm dark:prose-invert">
                {summary || "No analysis available yet. Start speaking or click 'Manual Summary'."}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    {:else}
      <MeetingHistory onSelect={handleSelectMeeting} />
    {/if}
  </div>

  <ThinkingPanel active={isThinking} />
</main>

<footer class="border-t px-4 py-1.5 text-[10px] text-muted-foreground flex justify-between shrink-0">
  <span>{isRecording ? t("statusRecording") : t("statusReady")}</span>
  <span>Bridge: {pingResult}</span>
</footer>
