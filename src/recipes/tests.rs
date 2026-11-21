use super::*;

fn vars<'a>(transcript: &'a str) -> Vars<'a> {
    Vars {
        id: "1790000000000-1001",
        title: "Acme intro",
        date: "2026-11-20 14:00",
        duration: "32 min",
        app: "Zoom",
        summary: "",
        transcript,
    }
}

#[test]
fn every_shipped_recipe_parses() {
    for (id, source) in DEFAULTS {
        let recipe = parse(id, Path::new("x.md"), source).unwrap_or_else(|e| panic!("{e}"));
        assert!(!recipe.prompt.is_empty(), "{id}");
        assert!(recipe.prompt.contains("{{transcript}}"), "{id} never passes the transcript");
        assert_eq!(recipe.command[0], "opencode", "{id}");
    }
}

#[test]
fn the_prompt_is_one_argument_and_nothing_runs_through_a_shell() {
    let source = "---\nname: T\ncommand: opencode run --title \"Hex: {{title}}\" {prompt}\nsends_to: X\n---\nSum up {{title}}:\n{{transcript}}";
    let recipe = parse("t", Path::new("t.md"), source).unwrap();
    let argv = recipe.argv(&vars("[00:00] Them: hi; rm -rf ~ {{title}}\n"));
    assert_eq!(argv[..4], ["opencode", "run", "--title", "Hex: Acme intro"]);
    assert_eq!(argv.len(), 5);
    // Placeholders inside the transcript stay as spoken.
    assert_eq!(argv[4], "Sum up Acme intro:\n[00:00] Them: hi; rm -rf ~ {{title}}\n");
}

#[test]
fn a_command_without_prompt_gets_it_last() {
    let source = "---\nname: T\ncommand: claude -p\n---\n{{transcript}}";
    let recipe = parse("t", Path::new("t.md"), source).unwrap();
    assert_eq!(recipe.argv(&vars("x")), ["claude", "-p", "x"]);
    assert_eq!(recipe.sends_to, "Whatever claude is set up to use");
}

#[test]
fn mistakes_are_named() {
    let err = |s: &str| parse("bad", Path::new("bad.md"), s).unwrap_err();
    assert!(err("no front matter").contains("starts with"));
    assert!(err("---\nname: x\n").contains("never closes"));
    assert!(err("---\nname: x\n---\nhi").contains("needs a command"));
    assert!(err("---\ncommand: x\n---\nhi").contains("needs a name"));
    assert!(err("---\nname: x\ncolour: red\ncommand: x\n---\n").contains("unknown key"));
}

/// Runs the shipped summary recipe for real through OpenCode on the made-up
/// demo call. Costs a model request, so it only runs when asked:
/// `cargo test -- --ignored summary_recipe`.
#[test]
#[ignore]
fn summary_recipe_runs_through_opencode() {
    let (recipes, _) = defaults();
    let recipe = recipes.iter().find(|r| r.id == "summary").unwrap();
    let d = crate::demo::sales_detail();
    let transcript = crate::hex::transcript_text(&d.entries);
    let argv = recipe.argv(&Vars { transcript: &transcript, ..vars("") });
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ended = crate::runner::run(&argv, &stop);
    println!("{}", ended.output);
    assert_eq!(ended.outcome, crate::activity::Outcome::Done, "{}", ended.output);
    assert!(ended.output.to_lowercase().contains("northwind"));
}

#[test]
fn an_empty_folder_gets_the_defaults_once() {
    let dir = std::env::temp_dir().join(format!("hex-meetings-recipes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (recipes, errors) = load(&dir);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(recipes.len(), DEFAULTS.len());
    std::fs::remove_file(dir.join("summary.md")).unwrap();
    let (recipes, _) = load(&dir);
    assert_eq!(recipes.len(), DEFAULTS.len() - 1, "a deleted default stays deleted");
    let _ = std::fs::remove_dir_all(&dir);
}
