use std::ops::RangeInclusive;

use chrono::{DateTime, Utc};
use md5::Digest;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{jfs::RevisionState, Path};

#[serde_as]
#[derive(Debug, Serialize)]
pub struct AllocReq<'a> {
    pub path: &'a Path,
    /// How many *more* bytes to allocate.
    pub bytes: u64,
    #[serde(with = "crate::serde::md5_hex")]
    pub md5: md5::Digest,
    pub conflict_handler: ConflictHandler,
    #[serde_as(as = "Option<serde_with::TimestampMilliSeconds<i64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,
    #[serde_as(as = "Option<serde_with::TimestampMilliSeconds<i64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConflictHandler {
    RejectConflicts,
    CreateNewRevision,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocRes {
    pub name: String,
    pub path: Path,
    pub state: RevisionState,
    pub upload_id: String,
    pub upload_url: String,
    pub bytes: u64,
    pub resume_pos: u64,
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteUploadRes {
    #[serde(with = "crate::serde::md5_hex")]
    pub md5: Digest,
    pub bytes: u64,
    pub content_id: String,
    pub path: Path,
    #[serde_as(as = "serde_with::TimestampMilliSeconds<i64>")]
    pub modified: DateTime<Utc>,
}

#[derive(Debug)]
pub struct IncompleteUploadRes {
    /// Range of the bytes uploaded now -- NOT the total bytes uploaded (for all chunks).
    pub range: RangeInclusive<u64>,
}

#[derive(Debug)]
pub enum UploadRes {
    Complete(CompleteUploadRes),
    Incomplete(IncompleteUploadRes),
}
