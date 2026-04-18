<script lang="ts">
  import { meetingsList, type Meeting } from "$lib/bridge";
  import { onMount } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "$lib/components/ui/card";
  import { Calendar, Clock, ChevronRight } from "lucide-svelte";

  let { onSelect } = $props();
  let meetings = $state<Meeting[]>([]);
  let loading = $state(true);

  async function loadMeetings() {
    loading = true;
    try {
      meetings = await meetingsList();
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  onMount(loadMeetings);

  function formatDate(iso: string) {
    return new Date(iso).toLocaleDateString(undefined, { 
      year: 'numeric', month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit'
    });
  }
</script>

<div class="flex flex-col gap-4 h-full">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-bold">Past Meetings</h2>
    <Button variant="outline" size="sm" onclick={loadMeetings}>Refresh</Button>
  </div>

  {#if loading}
    <p class="text-muted-foreground text-center py-8">Loading history...</p>
  {:else if meetings.length === 0}
    <div class="flex flex-col items-center justify-center py-12 border-2 border-dashed rounded-lg">
      <Calendar class="h-12 w-12 text-muted-foreground mb-4 opacity-20" />
      <p class="text-muted-foreground">No meetings found yet.</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each meetings as meeting}
        <Card 
          class="hover:border-primary/50 transition-colors cursor-pointer"
          onclick={() => onSelect(meeting)}
        >
          <CardHeader class="p-4">
            <div class="flex justify-between items-start">
              <CardTitle class="text-base line-clamp-1">{meeting.title}</CardTitle>
              <ChevronRight class="h-4 w-4 text-muted-foreground" />
            </div>
            <CardDescription class="flex items-center gap-1 text-xs mt-1">
              <Clock class="h-3 w-3" />
              {formatDate(meeting.started_at)}
            </CardDescription>
          </CardHeader>
        </Card>
      {/each}
    </div>
  {/if}
</div>
