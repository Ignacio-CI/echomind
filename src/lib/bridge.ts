import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const EVENTS = {
  AUDIO_LEVEL: "audio.level",
  TRANSCRIPT_PARTIAL: "transcript.partial",
  TRANSCRIPT_FINAL: "transcript.final",
  AI_TOKEN: "ai.token",
  SUMMARY_UPDATE: "summary.update",
} as const;

export function callPing(): Promise<string> {
  return invoke("ping");
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
