---
name: Support call to issues
description: One issue per problem the customer reported, with steps and a quote.
command: opencode run --archived --auto --title "Hex Meetings: {{title}}" {prompt}
sends_to: The model your OpenCode is set to use, then your issue tracker
---
Below is the transcript of a support call recorded with Hex. "You" is the support person who recorded it. "Them" is the customer's side.

Find every distinct problem the customer reported. For each one, create an issue with your issue tracker tools (Linear, Jira or GitHub, whichever you have). If you have none, write the issues as Markdown in your reply instead.

Each issue gets:

- A short title that names the problem.
- What the customer was trying to do and what happened instead.
- Steps to reproduce, if they can be worked out from the call.
- How badly it affects them: blocking, annoying, or cosmetic.
- One short quote from the transcript with its timestamp, as evidence.

Do not create an issue for questions that were fully answered on the call. When done, reply with a list of the issues you created and their links.

Call: {{title}} ({{app}}, {{date}}, {{duration}})

Transcript:

{{transcript}}
