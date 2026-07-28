//! End-to-end tests that run the real binary.
//!
//! Unit tests cover the pieces; these cover the thing a user actually invokes —
//! argument parsing, exit codes, which stream output lands on, and the JSON contract —
//! none of which can be verified by calling a function, because they are properties of
//! the process.
//!
//! Every test points `HOME`/`APPDATA` at a temporary directory. Without that the suite
//! would read and write the developer's real settings, profiles and export history, and
//! `history` would report their actual projects.

use std::path::Path;
use std::process::{Command, Output};

/// Path of the binary under test. Cargo builds it before running this target and hands
/// us the path, so this never runs a stale copy or a different installation.
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_codepack")
}

struct Sandbox {
    home: tempfile::TempDir,
    project: tempfile::TempDir,
    out: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(project.path().join("src")).unwrap();
        std::fs::write(project.path().join("src/main.py"), "print('hello')\n").unwrap();
        std::fs::write(project.path().join("README.md"), "# Demo\n").unwrap();
        std::fs::write(project.path().join("requirements.txt"), "requests\n").unwrap();
        Self {
            home: tempfile::tempdir().unwrap(),
            project,
            out: tempfile::tempdir().unwrap(),
        }
    }

    /// Adds a file containing a credential-shaped value. Synthetic and obviously fake.
    fn with_secret(self) -> Self {
        std::fs::write(
            self.project.path().join(".env"),
            concat!("API_KEY=", "totally-fake-value-0001\n"),
        )
        .unwrap();
        self
    }

    fn project_config(self, contents: &str) -> Self {
        std::fs::write(self.project.path().join(".codepack.toml"), contents).unwrap();
        self
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(binary());
        command
            .args(args)
            .env("HOME", self.home.path())
            .env("APPDATA", self.home.path())
            .env("LOCALAPPDATA", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path());
        command.output().unwrap()
    }

    fn project(&self) -> &Path {
        self.project.path()
    }

    fn out(&self) -> &Path {
        self.out.path()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout(output)).unwrap_or_else(|error| {
        panic!(
            "stdout was not valid JSON ({error}).\nstdout:\n{}\nstderr:\n{}",
            stdout(output),
            stderr(output)
        )
    })
}

fn code(output: &Output) -> i32 {
    output
        .status
        .code()
        .expect("process was killed by a signal")
}

// --- exit codes: the contract ROADMAP §3 fixes -------------------------------------

#[test]
fn a_clean_project_exits_zero() {
    let sandbox = Sandbox::new();
    assert_eq!(
        code(&sandbox.run(&["scan", &sandbox.project().display().to_string()])),
        0
    );
}

#[test]
fn critical_secrets_exit_three() {
    let sandbox = Sandbox::new().with_secret();
    let output = sandbox.run(&["scan", &sandbox.project().display().to_string()]);
    assert_eq!(
        code(&output),
        3,
        "a .env with a credential must gate a pipeline.\nstdout:\n{}",
        stdout(&output)
    );
}

#[test]
fn a_missing_project_exits_one_not_three() {
    // The distinction matters: 3 means "scanned successfully, found secrets". A failed
    // run reporting 3 would tell a pipeline the result is trustworthy.
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["scan", "definitely-not-a-directory-9f3b"]);
    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("definitely-not-a-directory-9f3b"));
}

#[test]
fn unknown_arguments_exit_two() {
    let sandbox = Sandbox::new();
    assert_eq!(code(&sandbox.run(&["--not-a-flag"])), 2);
    assert_eq!(code(&sandbox.run(&["frobnicate"])), 2);
    assert_eq!(
        code(&sandbox.run(&["scan", ".", "--safe-mode", "paranoid"])),
        2
    );
}

// --- the --json contract ------------------------------------------------------------

#[test]
fn every_command_emits_a_versioned_envelope() {
    let sandbox = Sandbox::new();
    let project = sandbox.project().display().to_string();

    for (args, command) in [
        (vec!["--json", "preview", project.as_str()], "preview"),
        (vec!["--json", "scan", project.as_str()], "scan"),
        (vec!["--json", "history"], "history"),
        (vec!["--json", "doctor"], "doctor"),
    ] {
        let output = sandbox.run(&args);
        let parsed = json(&output);
        assert_eq!(parsed["schema_version"], 1, "for {command}");
        assert_eq!(parsed["command"], command);
    }
}

#[test]
fn json_stdout_stays_parseable_even_though_the_export_logs_progress() {
    // The single most breakable property of `--json`: an export emits progress for
    // every step, and if any of it reached stdout the document would not parse.
    let sandbox = Sandbox::new();
    let output = sandbox.run(&[
        "--json",
        "export",
        &sandbox.project().display().to_string(),
        "--out",
        &sandbox.out().display().to_string(),
    ]);

    let parsed = json(&output);
    assert_eq!(parsed["command"], "export");
    assert_eq!(parsed["successful"], true);
    assert!(
        !stderr(&output).is_empty(),
        "progress should still be reported"
    );
}

#[test]
fn errors_go_to_stderr_so_stdout_never_carries_an_error_where_a_result_was_promised() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["--json", "scan", "definitely-not-a-directory-9f3b"]);

    assert_eq!(code(&output), 1);
    assert!(
        stdout(&output).trim().is_empty(),
        "stdout must stay empty on failure, got: {}",
        stdout(&output)
    );
    assert!(stderr(&output).contains("error") || stderr(&output).contains("does not exist"));
}

// --- export -------------------------------------------------------------------------

#[test]
fn export_writes_an_archive_and_records_it_in_history() {
    let sandbox = Sandbox::new();
    let project = sandbox.project().display().to_string();
    let out = sandbox.out().display().to_string();

    let exported = sandbox.run(&["--json", "export", &project, "--out", &out]);
    assert_eq!(code(&exported), 0);

    let report = json(&exported);
    let archive = report["result_path"].as_str().expect("an archive path");
    assert!(Path::new(archive).is_file(), "{archive} should exist");

    let history = json(&sandbox.run(&["--json", "history"]));
    let runs = history["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["successful"], true);
    assert_eq!(runs[0]["run_id"], report["run_id"]);
}

#[test]
fn an_exported_bundle_does_not_contain_the_secret_file() {
    // Invariant I3's whole promise, verified through the binary a user runs.
    let sandbox = Sandbox::new().with_secret();
    let exported = sandbox.run(&[
        "--json",
        "export",
        &sandbox.project().display().to_string(),
        "--out",
        &sandbox.out().display().to_string(),
    ]);

    let report = json(&exported);
    let archive = report["result_path"].as_str().unwrap();
    let file = std::fs::File::open(archive).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();

    let names: Vec<String> = (0..zip.len())
        .map(|index| zip.by_index(index).unwrap().name().to_string())
        .collect();
    assert!(
        !names.iter().any(|name| name.ends_with(".env")),
        "the bundle must not carry .env: {names:?}"
    );
}

// --- preview ------------------------------------------------------------------------

#[test]
fn preview_reports_the_sensitive_file_without_writing_anything() {
    let sandbox = Sandbox::new().with_secret();
    let before = listing(sandbox.project());

    let output = sandbox.run(&[
        "--json",
        "preview",
        &sandbox.project().display().to_string(),
    ]);
    let report = json(&output);

    assert_eq!(code(&output), 0);
    let flagged = report["sensitive_exclusions"].as_array().unwrap();
    assert_eq!(flagged.len(), 1);
    assert!(flagged[0]["path"].as_str().unwrap().contains(".env"));

    assert_eq!(
        before,
        listing(sandbox.project()),
        "preview must not create or remove anything in the project"
    );
}

#[test]
fn preview_lists_files_only_when_asked() {
    let sandbox = Sandbox::new();
    let project = sandbox.project().display().to_string();

    let quiet = json(&sandbox.run(&["--json", "preview", &project]));
    assert!(quiet.get("files").is_none());

    let listed = json(&sandbox.run(&["--json", "preview", &project, "--list-files"]));
    assert!(!listed["files"].as_array().unwrap().is_empty());
}

// --- configuration layering ---------------------------------------------------------

#[test]
fn a_project_config_changes_behaviour_and_is_reported() {
    let sandbox = Sandbox::new()
        .with_secret()
        .project_config("safe_export_mode = \"full\"\n");

    let report = json(&sandbox.run(&[
        "--json",
        "preview",
        &sandbox.project().display().to_string(),
    ]));

    assert_eq!(report["safe_mode"], "full");
    assert!(
        report["sensitive_exclusions"]
            .as_array()
            .unwrap()
            .is_empty(),
        "in full mode nothing is excluded on safety grounds"
    );
    assert!(
        report["resolution"]["project_config"]
            .as_str()
            .unwrap()
            .ends_with(".codepack.toml")
    );
}

#[test]
fn a_flag_overrides_the_project_config() {
    let sandbox = Sandbox::new().project_config("safe_export_mode = \"full\"\n");
    let report = json(&sandbox.run(&[
        "--json",
        "preview",
        &sandbox.project().display().to_string(),
        "--safe-mode",
        "safe",
    ]));
    assert_eq!(report["safe_mode"], "safe");
}

#[test]
fn a_broken_project_config_fails_loudly_rather_than_being_ignored() {
    let sandbox = Sandbox::new().project_config("safe_moed = \"safe\"\n");
    let output = sandbox.run(&["preview", &sandbox.project().display().to_string()]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains(".codepack.toml"),
        "the message must point at the file: {}",
        stderr(&output)
    );
}

#[test]
fn an_unknown_preset_is_rejected_and_names_the_real_ones() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&[
        "preview",
        &sandbox.project().display().to_string(),
        "--preset",
        "clade",
    ]);

    assert_eq!(code(&output), 1);
    let message = stderr(&output);
    assert!(message.contains("clade"));
    assert!(message.contains("available presets"));
}

#[test]
fn a_budget_drops_files_and_says_so() {
    let sandbox = Sandbox::new();
    for index in 0..8 {
        std::fs::write(
            sandbox.project().join(format!("filler_{index}.txt")),
            "x".repeat(4_000),
        )
        .unwrap();
    }

    let report = json(&sandbox.run(&[
        "--json",
        "preview",
        &sandbox.project().display().to_string(),
        "--budget",
        "2k",
    ]));

    assert!(
        report["dropped_by_budget"].as_u64().unwrap() > 0,
        "a 2k budget cannot fit 32k of filler: {report}"
    );
}

// --- history and doctor -------------------------------------------------------------

#[test]
fn history_on_a_fresh_machine_is_empty_rather_than_an_error() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["--json", "history"]);
    assert_eq!(code(&output), 0);
    assert!(json(&output)["runs"].as_array().unwrap().is_empty());
}

#[test]
fn history_for_a_never_exported_project_lists_nothing_and_creates_no_record() {
    let sandbox = Sandbox::new();
    let project = sandbox.project().display().to_string();

    let listed = json(&sandbox.run(&["--json", "history", "--path", &project]));
    assert!(listed["runs"].as_array().unwrap().is_empty());

    // The `runs` list alone cannot prove the second half of this test's name:
    // `list_export_runs` joins runs to projects, so a stray project row carrying no
    // runs would never appear there. Ask the table directly.
    for database in find_databases(sandbox.home.path()) {
        let conn = rusqlite::Connection::open(&database).unwrap();
        let projects: i64 = conn
            .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            projects,
            0,
            "a read-only command registered a project in {}",
            database.display()
        );
    }
}

#[test]
fn doctor_states_the_privacy_guarantee_and_lists_the_presets() {
    let sandbox = Sandbox::new();
    let report = json(&sandbox.run(&["--json", "doctor"]));

    assert!(report["network_access"].as_str().unwrap().contains("local"));
    assert_eq!(report["presets"].as_array().unwrap().len(), 5);
    assert_eq!(report["project_config_file"], ".codepack.toml");
}

#[test]
fn help_and_version_work_and_exit_zero() {
    let sandbox = Sandbox::new();
    for args in [vec!["--help"], vec!["--version"], vec!["export", "--help"]] {
        let output = sandbox.run(&args);
        assert_eq!(code(&output), 0, "for {args:?}");
        assert!(!stdout(&output).is_empty(), "for {args:?}");
    }
}

#[test]
fn completions_prints_a_script_for_every_supported_shell() {
    let sandbox = Sandbox::new();
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        let output = sandbox.run(&["completions", shell]);
        assert_eq!(code(&output), 0, "for shell {shell}");
        let script = stdout(&output);
        assert!(!script.is_empty(), "for shell {shell}");
        assert!(
            script.contains("codepack"),
            "the generated script should reference the binary name for {shell}"
        );
    }
}

/// Every path under `root`, recursively. A non-recursive listing would not notice a
/// file written into a subdirectory, which is exactly what "writes nothing anywhere"
/// has to rule out.
fn listing(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap())
            .collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), &format!("{name}/"), found);
            }
            found.push(name);
        }
    }
    let mut found = Vec::new();
    walk(root, "", &mut found);
    found
}

/// Every `.db` under the sandboxed home, so a test need not know where `AppPaths`
/// decided to put it on this platform.
fn find_databases(root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(find_databases(&path));
        } else if path.extension().is_some_and(|ext| ext == "db") {
            found.push(path);
        }
    }
    found
}

// --- gaps the review found in this suite's own coverage ------------------------------

#[test]
fn an_export_whose_bundle_holds_critical_findings_exits_three() {
    // `scan`'s exit-3 path was covered; export's was not, and the CI acceptance step
    // only *tolerates* 3 rather than demonstrating it.
    let sandbox = Sandbox::new();
    // Inside a source file rather than a `.pem`: safe mode excludes a `.pem` outright,
    // so the bundle would be clean and exiting 0 would be right. What this test needs is
    // a critical finding in a file that really is exported.
    std::fs::write(
        sandbox.project().join("bootstrap.py"),
        "KEY = \"\"\"-----BEGIN RSA PRIVATE KEY-----\nnot-a-real-key\n\"\"\"\n",
    )
    .unwrap();

    let output = sandbox.run(&[
        "--json",
        "export",
        &sandbox.project().display().to_string(),
        "--out",
        &sandbox.out().display().to_string(),
    ]);

    let report = json(&output);
    assert!(
        report["critical_findings"].as_u64().unwrap() > 0,
        "expected the PEM header to be reported: {report}"
    );
    assert_eq!(code(&output), 3);
}

#[test]
fn a_preset_is_applied_and_reported() {
    let sandbox = Sandbox::new();
    let report = json(&sandbox.run(&[
        "--json",
        "preview",
        &sandbox.project().display().to_string(),
        "--preset",
        "Security Audit",
    ]));

    assert_eq!(report["resolution"]["preset"], "Security Audit");
    assert_eq!(report["profile"], "security");
    assert_eq!(report["safe_mode"], "safe");
}

#[test]
fn an_unknown_profile_is_rejected_the_way_an_unknown_preset_is() {
    // Without validation, apply_custom_profile's legacy fallback turns a typo into a
    // *wider* export than the user asked for, while still reporting the typed name.
    let sandbox = Sandbox::new();
    let output = sandbox.run(&[
        "preview",
        &sandbox.project().display().to_string(),
        "--profile",
        "minimul",
    ]);

    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("minimul"));
    assert!(stderr(&output).contains("available profiles"));
}

#[test]
fn a_diff_flag_reaches_the_plan() {
    let sandbox = Sandbox::new();
    let report = json(&sandbox.run(&[
        "--json",
        "preview",
        &sandbox.project().display().to_string(),
        "--diff",
        "uncommitted",
    ]));
    // The fixture is not a git repository, so codepack-diff degrades to "all" and says
    // so; the point here is that the flag was carried into the plan at all.
    assert!(report["diff_mode"].is_string());
}

#[test]
fn the_default_output_location_never_lands_inside_the_project() {
    // Invariant I2. This defaulted to the current directory at first, which for the
    // documented `cd project && codepack export` is the project itself: two runs left
    // two archives in the tree, and the second had to skip the first one's output as if
    // it were source.
    let sandbox = Sandbox::new();
    let before = listing(sandbox.project());

    let output = Command::new(binary())
        .args(["export", "--json"])
        .current_dir(sandbox.project())
        .env("HOME", sandbox.home.path())
        .env("APPDATA", sandbox.home.path())
        .env("LOCALAPPDATA", sandbox.home.path())
        .env("USERPROFILE", sandbox.home.path())
        .env("XDG_CONFIG_HOME", sandbox.home.path())
        .output()
        .unwrap();

    assert_eq!(code(&output), 0, "stderr:\n{}", stderr(&output));
    assert_eq!(
        before,
        listing(sandbox.project()),
        "the export wrote into the project it was exporting"
    );
}

#[test]
fn an_explicit_out_inside_the_project_is_refused() {
    let sandbox = Sandbox::new();
    let inside = sandbox.project().join("dist");
    let output = sandbox.run(&[
        "export",
        &sandbox.project().display().to_string(),
        "--out",
        &inside.display().to_string(),
    ]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("refusing"),
        "stderr:\n{}",
        stderr(&output)
    );
}

// --- `sanitize` (the "Sterile copy" standalone action) -----------------------------

#[test]
fn sanitize_strips_comments_and_redacts_a_planted_secret() {
    let sandbox = Sandbox::new().with_secret();
    let output = sandbox.run(&[
        "sanitize",
        "--source",
        &sandbox.project().display().to_string(),
        "--out",
        &sandbox.out().display().to_string(),
    ]);

    assert_eq!(code(&output), 0, "stderr:\n{}", stderr(&output));

    // Quote style may differ (`ruff`/`black`, if present on the developer's `PATH`,
    // normalise quotes) — the content, not the exact formatter output, is what matters
    // here; formatter-specific behavior is covered in `codepack-sanitize`'s own tests.
    let written = std::fs::read_to_string(sandbox.out().join("src/main.py")).unwrap();
    assert!(
        written.contains("print(") && written.contains("hello"),
        "written content: {written:?}"
    );

    // .env is excluded by safe mode, exactly like a normal export, and never appears in
    // the destination folder at all.
    assert!(!sandbox.out().join(".env").exists());

    assert!(sandbox.out().join("STERILE_COPY_REPORT.json").is_file());
    assert!(sandbox.out().join("STERILE_COPY_REPORT.md").is_file());
}

#[test]
fn sanitize_destination_inside_the_source_is_refused() {
    let sandbox = Sandbox::new();
    let inside = sandbox.project().join("sterile");
    let output = sandbox.run(&[
        "sanitize",
        "--source",
        &sandbox.project().display().to_string(),
        "--out",
        &inside.display().to_string(),
    ]);

    assert_eq!(code(&output), 1);
    assert!(
        stderr(&output).contains("refusing"),
        "stderr:\n{}",
        stderr(&output)
    );
}

#[test]
fn sanitize_json_output_is_a_versioned_envelope() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&[
        "sanitize",
        "--source",
        &sandbox.project().display().to_string(),
        "--out",
        &sandbox.out().display().to_string(),
        "--json",
    ]);

    assert_eq!(code(&output), 0, "stderr:\n{}", stderr(&output));
    let payload = json(&output);
    assert_eq!(payload["command"], "sanitize");
    assert!(payload["summary"]["total_files"].as_u64().unwrap() > 0);
}
