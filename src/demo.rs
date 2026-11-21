//! Made-up meetings for screenshots, scenes and tests. None of this is a
//! real call.

use crate::activity::{Outcome, Run};
use crate::app::{App, HexStatus};
use crate::hex::{Detail, Entry, Meeting, Notes};

const HOUR: i64 = 3_600_000;

fn meeting(id: &str, title: &str, ago_hours: i64, minutes: u64, app: Option<&str>) -> Meeting {
    Meeting {
        id: id.into(),
        title: title.into(),
        timestamp_ms: chrono::Utc::now().timestamp_millis() - ago_hours * HOUR,
        duration_ms: minutes * 60_000,
        application: app.map(Into::into),
        status: "complete".into(),
        in_progress: false,
        transcript_source: "final".into(),
        warnings: Vec::new(),
    }
}

fn entries(lines: &[(&str, u64, &str)]) -> Vec<Entry> {
    lines
        .iter()
        .map(|(who, s, text)| Entry {
            source: if *who == "you" { "microphone" } else { "system" }.into(),
            start_ms: s * 1000,
            end_ms: s * 1000 + 4000,
            text: (*text).into(),
        })
        .collect()
}

pub fn meetings() -> Vec<Meeting> {
    vec![
        meeting("demo-sales", "Northwind intro call", 2, 34, Some("Google Meet")),
        meeting("demo-support", "Fabrikam: export failing", 5, 18, Some("Zoom")),
        meeting("demo-standup", "Team standup", 26, 12, Some("Slack")),
        meeting("demo-board", "Board prep with Alex", 50, 47, None),
        meeting("demo-hiring", "Design candidate, round 2", 120, 41, Some("Microsoft Teams")),
    ]
}

pub fn sales_detail() -> Detail {
    Detail {
        meeting: meetings().remove(0),
        notes: Some(Notes {
            title: "Northwind Pilot Scope".into(),
            summary: "Northwind wants a pilot for their EU support team before Q1 and needs data to stay in the EU.".into(),
        }),
        entries: entries(&[
            ("them", 2, "Thanks for making time. I'm Priya, I run support operations at Northwind, and Jonas from IT is here too."),
            ("you", 11, "Great to meet you both. What made you reach out now?"),
            ("them", 15, "We record almost none of our customer calls today. Legal is nervous about where recordings end up, so people take notes by hand and half of it gets lost."),
            ("them", 27, "If transcripts stayed on the agent's laptop or at least inside the EU, that objection mostly goes away."),
            ("you", 38, "That's exactly how this works. Recording and transcription happen on the Mac, and you choose where a summary goes afterwards."),
            ("them", 49, "Jonas here. Our worry is the summary step. Which model sees the transcript?"),
            ("you", 56, "Whichever one you configure. Teams usually point it at an EU-hosted model they already have a contract with."),
            ("them", 66, "Okay. Budget-wise we have something set aside for a pilot this quarter, around twenty seats."),
            ("you", 74, "Twenty seats works. Shall I send a pilot plan by Friday, with the data-flow diagram for your legal team?"),
            ("them", 82, "Yes, please. And include how deletion works, legal will ask about that first."),
        ]),
    }
}

/// An app with the demo meetings loaded and the sales call open.
pub fn app() -> App {
    let (recipes, _) = crate::recipes::defaults();
    App {
        offline: true,
        hex: Some(HexStatus::Found { path: "/Applications/Hex.app/Contents/MacOS/hex".into(), version: Some("3.4.0".into()) }),
        meetings: meetings(),
        meetings_loaded: true,
        selected: Some("demo-sales".into()),
        detail: Some(sales_detail()),
        recipes,
        activity: vec![
            Run {
                at_ms: chrono::Utc::now().timestamp_millis() - 4 * HOUR,
                meeting_id: "demo-support".into(),
                meeting_title: "Fabrikam: export failing".into(),
                recipe: "Support call to issues".into(),
                sends_to: "The model your OpenCode is set to use, then your issue tracker".into(),
                program: "opencode".into(),
                outcome: Outcome::Done,
                output: "- **CSV export times out above 50k rows**: LIN-412\n- **Exported dates use US format**: LIN-413".into(),
            },
            Run {
                at_ms: chrono::Utc::now().timestamp_millis() - 25 * HOUR,
                meeting_id: "demo-standup".into(),
                meeting_title: "Team standup".into(),
                recipe: "Summary and action items".into(),
                sends_to: "The model your OpenCode is set to use".into(),
                program: "opencode".into(),
                outcome: Outcome::Failed,
                output: "opencode failed (exit 1).\n\nNo provider is configured.".into(),
            },
        ],
        ..App::default()
    }
}

/// What the sales call's summary recipe would give back.
pub fn summary_run() -> Run {
    Run {
        at_ms: chrono::Utc::now().timestamp_millis() - 60_000,
        meeting_id: "demo-sales".into(),
        meeting_title: "Northwind intro call".into(),
        recipe: "Summary and action items".into(),
        sends_to: "The model your OpenCode is set to use".into(),
        program: "opencode".into(),
        outcome: Outcome::Done,
        output: "**Summary**\n\nNorthwind (Priya, support operations; Jonas, IT) records few customer calls because legal is worried about where recordings go. Local transcription with an EU-hosted summary model addresses that. They have budget for a 20-seat pilot this quarter.\n\n**Action items**\n\n- [ ] You: send a pilot plan by Friday, with a data-flow diagram\n- [ ] You: explain how deletion works\n- [ ] Them: share the plan with legal\n\n**Open questions**\n\n- Which EU model provider Northwind already has a contract with.".into(),
    }
}
