import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const EVENTS = {
  AUDIO_LEVEL: "audio:level",
  TRANSCRIPT_PARTIAL: "transcript:partial",
  TRANSCRIPT_FINAL: "transcript:final",
  AI_TOKEN: "ai:token",
  SUMMARY_UPDATE: "summary:update",
  AI_STATUS: "ai:status",
} as const;

export function callPing(): Promise<string> {
  return invoke("ping");
}

export function modelsPrepare(): Promise<void> {
  return invoke("models_prepare");
}

export function modelsStatus(): Promise<boolean> {
  return invoke("models_status");
}

export function audioListDevices(): Promise<{ name: string }[]> {
  return invoke("audio_list_devices");
}

export function audioStart(): Promise<void> {
  return invoke("audio_start");
}

export function transcriptStart(cb: (text: string) => void): Promise<() => void> {
  return Promise.all([
    onTranscriptPartial(cb),
    onTranscriptFinal(cb),
  ]).then(([u1, u2]) => () => { u1(); u2(); });
}

export function audioStop(): Promise<void> {
  return invoke("audio_stop");
}

export function onAudioLevel(cb: (level: number) => void): Promise<UnlistenFn> {
  return listen<number>(EVENTS.AUDIO_LEVEL, (e) => cb(e.payload));
}

export function onTranscriptPartial(cb: (text: string) => void): Promise<UnlistenFn> {
  return listen<string>(EVENTS.TRANSCRIPT_PARTIAL, (e) => cb(e.payload));
}

export function onTranscriptFinal(cb: (text: string) => void): Promise<UnlistenFn> {
  return listen<string>(EVENTS.TRANSCRIPT_FINAL, (e) => cb(e.payload));
}

export function onAiToken(cb: (token: string) => void): Promise<UnlistenFn> {
  return listen<string>(EVENTS.AI_TOKEN, (e) => cb(e.payload));
}

export function onSummaryUpdate(cb: (summary: string) => void): Promise<UnlistenFn> {
  return listen<string>(EVENTS.SUMMARY_UPDATE, (e) => cb(e.payload));
}

export function onAiStatus(cb: (status: string) => void): Promise<UnlistenFn> {
  return listen<string>(EVENTS.AI_STATUS, (e) => cb(e.payload));
}

export function aiGenerate(prompt: string, locale: string): Promise<void> {
  return invoke("ai_generate", { prompt, locale });
}

export function aiCancel(): Promise<void> {
  return invoke("ai_cancel");
}

export function meetingSummarize(locale: string): Promise<void> {
  return invoke("meeting_summarize", { locale });
}

export type Meeting = {
  id: string;
  title: string;
  started_at: string;
  ended_at?: string;
  locale: string;
};

export type TranscriptEntry = {
  id?: number;
  meeting_id: string;
  t_start: number;
  t_end: number;
  text: string;
  is_final: boolean;
};

export function meetingsList(): Promise<Meeting[]> {
  return invoke("meetings_list");
}

export function meetingCreate(id: string, title: string, locale: string): Promise<void> {
  return invoke("meeting_create", { id, title, locale });
}

export function meetingGetTranscript(id: string): Promise<TranscriptEntry[]> {
  return invoke("meeting_get_transcript", { id });
}

export function meetingGetSummary(id: string): Promise<string | null> {
  return invoke("meeting_get_summary", { id });
}

export function settingsSet(key: string, value: string): Promise<void> {
  return invoke("settings_set", { key, value });
}

export function settingsGet(key: string): Promise<string | null> {
  return invoke("settings_get", { key });
}
