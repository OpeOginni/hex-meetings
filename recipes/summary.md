---
name: Summary and action items
description: A short summary, the decisions, and who does what next.
command: opencode run --archived --title "Hex Meetings: {{title}}" {prompt}
sends_to: The model your OpenCode is set to use
---
Below is the transcript of a call recorded with Hex. "You" is the person who recorded it. "Them" is everyone else on the call, so several people may speak as "Them".

Write, in Markdown:

1. **Summary**: 3 to 5 sentences.
2. **Decisions**: what was agreed.
3. **Action items**: one line each, as `- [ ] Owner: task`. Use You, Them, or a name if one is said.
4. **Open questions**: what is still unclear.

Use only what the transcript says. If something is unclear, say so instead of guessing. Reply with the Markdown only.

Call: {{title}} ({{app}}, {{date}}, {{duration}})

Transcript:

{{transcript}}
