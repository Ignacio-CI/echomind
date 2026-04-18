<script lang="ts">
  let { level } = $props();
</script>

<div class="flex items-center gap-2 w-48">
  <div class="relative h-2 flex-1 rounded-full bg-muted overflow-hidden">
    <div
      class="h-full rounded-full bg-primary transition-all duration-75"
      style="width: {Math.min(level * 300, 100)}%"
    ></div>
  </div>
  <span class="text-xs text-muted-foreground w-8 text-right">{Math.round(level * 100)}%</span>
</div>
