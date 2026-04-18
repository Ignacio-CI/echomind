const SYSTEM_PROMPT: &str = "You are a professional meeting secretary. Summarize meetings and extract action items. Be concise and objective. Use the same language as the transcript.";

pub fn get_summary_prompt(transcript: &str, locale: &str) -> String {
    let lang = if locale == "es" { "Spanish" } else { "English" };
    let user_msg = format!(
        "{}\n\nSummarize the following meeting transcript in {}:\n\n{}\n\nSummary and Action Items:",
        SYSTEM_PROMPT, lang, transcript
    );
    // Gemma 3 instruction format
    format!("<start_of_turn>user\n{}<end_of_turn>\n<start_of_turn>model\n", user_msg)
}
