use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    RunStart(RunStart),
    File(FileEvent),
    RunEnd(RunEnd),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStart {
    pub schema_version: String,
    pub tool_version: String,
    pub mode: String, // "cli" or "apply"
    pub input_mode: String, // "args", "stdin-paths", "stdin-text", "rg-json", "files0", "manifest"
    pub transaction_mode: String, // "all" or "file"
    pub dry_run: bool,
    pub validate_only: bool,
    pub no_write: bool,
    pub policies: Policies,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policies {
    pub require_match: bool,
    pub expect: Option<usize>,
    pub fail_on_change: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileEvent {
    Success {
        path: PathBuf,
        modified: bool,
        replacements: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        diff: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        generated_content: Option<String>,
    },
    Skipped {
        path: PathBuf,
        reason: SkipReason,
    },
    Error {
        path: PathBuf,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    Binary,
    Symlink,
    GlobExclude,
    NotModified, // Maybe not needed if we have Success with modified: false, but sometimes useful to be explicit if filtered out?
                 // Actually the TODO says "changed/skipped/error stats + reason enums".
                 // "NotModified" is usually a Success case with 0 replacements.
                 // "Skipped" usually implies we didn't even try to replace because of some property of the file.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEnd {
    pub total_files: usize,
    pub total_modified: usize,
    pub total_replacements: usize,
    pub has_errors: bool,
    pub policy_violation: Option<String>,
    pub exit_code: i32,
}
