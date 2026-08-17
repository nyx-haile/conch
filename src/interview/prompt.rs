const BASE: &str = "You are Conch, a warm, curious interviewer. You are speaking live with the user about the project summarized below.\n\n\
Style:\n\
- Freeform and conversational. Follow interesting threads.\n\
- Gently steer to cover the angles in the brief. Do not read them off as a list.\n\
- Keep turns short — 2-4 sentences max. Brevity keeps tempo when speaking.\n\
- Never narrate what you are about to do. Just do it.\n\n\
Tool use:\n\
- Call the `end_session` tool when the angles are meaningfully covered or the user signals they are done. Provide a one-line `reason`. The system will play your closing turn, then exit.\n\n\
Interview brief follows.\n";

/// Assemble the interview system prompt.
///
/// `corrections` is the prompt-memory block from
/// [`crate::stt::corrections::CorrectionStore::prompt_memory_text`] — personal
/// vocabulary that speech-to-text reliably mangles. It is appended last so it
/// stays close to the live transcript in the context window.
pub fn compose_system_prompt(
    brief: &str,
    brand: Option<&str>,
    corrections: Option<&str>,
) -> String {
    let mut out = String::with_capacity(BASE.len() + brief.len() + 512);
    out.push_str(BASE);
    out.push_str("\n---\n");
    out.push_str(brief.trim());
    if let Some(brand) = brand {
        out.push_str("\n\n---\nVoice & tone guidance:\n");
        out.push_str(brand.trim());
    }
    if let Some(corrections) = corrections {
        let corrections = corrections.trim();
        if !corrections.is_empty() {
            out.push_str("\n\n---\n");
            out.push_str(corrections);
        }
    }
    out
}
