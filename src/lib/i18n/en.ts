export const en = {
  appTitle: "EchoMind",
  startRecording: "Start Recording",
  stopRecording: "Stop Recording",
  starting: "Starting AI...",
  preparing: "Preparing...",
  statusReady: "Ready",
  statusRecording: "Recording…",
  statusTranscribing: "Transcribing…",
  meetingHistory: "Meeting History",
  welcomeTitle: "Welcome to EchoMind",
  welcomeDesc: "Start recording to begin your AI-powered meeting analysis.",
  thinkingMode: "Thinking Mode",
  thinkingModeDesc: "AI is analyzing your meeting…",
  summary: "Summary",
  actionItems: "Action Items",
  language: "Language",
  errorGeneric: "Something went wrong. Please try again.",
  cancel: "Cancel",
  noMeetings: "No meetings yet. Start recording to create one.",
} as const;

export type TranslationKey = keyof typeof en;
