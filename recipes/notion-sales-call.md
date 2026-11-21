---
name: Sales call to Notion
description: A deal note in Notion with the customer's needs, objections and next steps.
command: opencode run --archived --auto --title "Hex Meetings: {{title}}" {prompt}
sends_to: The model your OpenCode is set to use, then your Notion
---
Below is the transcript of a sales call recorded with Hex. "You" is the salesperson who recorded it. "Them" is the prospect's side, so several people may speak as "Them".

Using your Notion tools, create one page in the Notion database called "Sales calls". If there is no such database, create the page at the top level of the workspace instead. If you have no Notion tools at all, reply "No Notion connection found" and stop.

Title the page "{{title}} ({{date}})" and give it these sections:

- **Company and people**: who was on the call and their roles, if said.
- **What they need**: their problems and goals, in their own words where possible.
- **Budget and timeline**: only what was actually said.
- **Objections**: concerns they raised and how they were answered.
- **Next steps**: as a to-do list with owners and dates if given.

Use only what the transcript says. Do not paste the whole transcript into Notion. When done, reply with the page's link and nothing else.

Transcript ({{app}}, {{duration}}):

{{transcript}}
