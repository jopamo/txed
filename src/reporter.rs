use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::events::{Event, RunStart, FileEvent, RunEnd, Policies, SkipReason};
use crate::model::Pipeline;

/// Result of processing a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileResult {
    /// Path to the file.
    pub path: PathBuf,
    /// Whether the file was modified.
    pub modified: bool,
    /// Number of replacements performed.
    pub replacements: usize,
    /// Error, if any.
    pub error: Option<String>,
    /// Reason for skipping the file, if skipped.
    pub skipped: Option<String>,
    /// Diff lines (if dry_run or preview).
    pub diff: Option<String>,
}

/// Overall execution report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    /// Results for each file.
    pub files: Vec<FileResult>,
    /// Total files processed.
    pub total: usize,
    /// Total files modified.
    pub modified: usize,
    /// Total replacements performed.
    pub replacements: usize,
    /// Whether dry-run mode was active.
    pub dry_run: bool,
    /// Whether validate-only mode was active.
    pub validate_only: bool,
    /// Whether any errors occurred.
    pub has_errors: bool,
    /// Policy violation message (if any).
    pub policy_violation: Option<String>,
}

impl Report {
    /// Create a new empty report.
    pub fn new(dry_run: bool, validate_only: bool) -> Self {
        Self {
            files: Vec::new(),
            total: 0,
            modified: 0,
            replacements: 0,
            dry_run,
            validate_only,
            has_errors: false,
            policy_violation: None,
        }
    }

    /// Add a file result.
    pub fn add_result(&mut self, result: FileResult) {
        self.total += 1;
        if result.modified {
            self.modified += 1;
        }
        self.replacements += result.replacements;
        if result.error.is_some() {
            self.has_errors = true;
        }
        self.files.push(result);
    }

    /// Print report in human-readable format.
    pub fn print_human(&self) {
        if let Some(msg) = &self.policy_violation {
            eprintln!("Policy Error: {}", msg);
        }

        if self.validate_only {
            println!("VALIDATION RUN - No files were written.");
        } else if self.dry_run {
            println!("DRY RUN - No files were written.");
        }
        println!("Processed {} files, modified {}, {} replacements.",
                 self.total, self.modified, self.replacements);
        for file in &self.files {
            if let Some(err) = &file.error {
                println!("  {}: ERROR - {}", file.path.display(), err);
            } else if let Some(reason) = &file.skipped {
                println!("  {}: skipped ({})", file.path.display(), reason);
            } else if file.modified {
                println!("  {}: modified ({} replacements)", file.path.display(), file.replacements);
                if let Some(diff) = &file.diff {
                    println!("{}", diff);
                }
            } else {
                println!("  {}: no changes", file.path.display());
            }
        }
    }

    /// Print report in summary format (human-readable, but no diffs).
    pub fn print_summary(&self) {
        if let Some(msg) = &self.policy_violation {
            eprintln!("Policy Error: {}", msg);
        }

        if self.validate_only {
            println!("VALIDATION RUN - No files were written.");
        } else if self.dry_run {
            println!("DRY RUN - No files were written.");
        }
        println!("Processed {} files, modified {}, {} replacements.",
                 self.total, self.modified, self.replacements);
        for file in &self.files {
            if let Some(err) = &file.error {
                println!("  {}: ERROR - {}", file.path.display(), err);
            } else if let Some(reason) = &file.skipped {
                println!("  {}: skipped ({})", file.path.display(), reason);
            } else if file.modified {
                println!("  {}: modified ({} replacements)", file.path.display(), file.replacements);
                // Diff is explicitly omitted in summary format
            } else {
                println!("  {}: no changes", file.path.display());
            }
        }
    }

    /// Print only errors (for --quiet).
    pub fn print_errors_only(&self) {
        if let Some(msg) = &self.policy_violation {
            eprintln!("Policy Error: {}", msg);
        }
        for file in &self.files {
             if let Some(err) = &file.error {
                eprintln!("  {}: ERROR - {}", file.path.display(), err);
            }
        }
    }

    /// Determine the appropriate exit code for this report.
    pub fn exit_code(&self) -> i32 {
        use crate::exit_codes;
        if self.policy_violation.is_some() {
            exit_codes::POLICY_VIOLATION
        } else if self.has_errors {
            exit_codes::ERROR
        } else {
            exit_codes::SUCCESS
        }
    }

    /// Print report as JSON events.
    pub fn print_json(&self, pipeline: &Pipeline, tool_version: &str, mode: &str, input_mode: &str) {
        if input_mode == "stdin-text" {
            // Ensure JSON starts on a new line if content was printed without trailing newline
            println!();
        }
        let start = RunStart {
            schema_version: "1".into(),
            tool_version: tool_version.into(),
            mode: mode.into(),
            input_mode: input_mode.into(),
            transaction_mode: format!("{:?}", pipeline.transaction).to_lowercase(),
            dry_run: pipeline.dry_run,
            validate_only: pipeline.validate_only,
            no_write: pipeline.no_write,
            policies: Policies {
                require_match: pipeline.require_match,
                expect: pipeline.expect,
                fail_on_change: pipeline.fail_on_change,
            },
        };
        println!("{}", serde_json::to_string(&Event::RunStart(start)).unwrap());

        for file in &self.files {
            let event = if let Some(err) = &file.error {
                FileEvent::Error {
                    path: file.path.clone(),
                    message: err.clone(),
                }
            } else if let Some(reason) = &file.skipped {
                 let reason_enum = match reason.as_str() {
                    "binary file" => SkipReason::Binary,
                    "symlink" => SkipReason::Symlink,
                    "glob exclude" => SkipReason::GlobExclude,
                     _ => SkipReason::NotModified, 
                };
                FileEvent::Skipped {
                    path: file.path.clone(),
                    reason: reason_enum,
                }
            } else {
                FileEvent::Success {
                    path: file.path.clone(),
                    modified: file.modified,
                    replacements: file.replacements,
                    diff: file.diff.clone(),
                }
            };
            println!("{}", serde_json::to_string(&Event::File(event)).unwrap());
        }

        let end = RunEnd {
            total_files: self.total,
            total_modified: self.modified,
            total_replacements: self.replacements,
            has_errors: self.has_errors,
            policy_violation: self.policy_violation.clone(),
            exit_code: self.exit_code(),
        };
        println!("{}", serde_json::to_string(&Event::RunEnd(end)).unwrap());
    }

    /// Print report in Agent-friendly XML format.
    pub fn print_agent(&self) {
        for file in &self.files {
            println!("<file path=\"{}\">", file.path.display());
            if let Some(err) = &file.error {
                println!("ERROR: {}", err);
            } else if let Some(reason) = &file.skipped {
                println!("SKIPPED: {}", reason);
            } else if let Some(diff) = &file.diff {
                println!("{}", diff);
            } else if file.modified {
                 println!("(modified)");
            } else {
                 println!("(no changes)");
            }
            println!("</file>");
        }
    }
}
