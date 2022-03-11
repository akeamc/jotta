//! Utilities for the API at `api.jottacloud.com/files/v1`.
use std::ops::RangeInclusive;

use chrono::{DateTime, Utc};
use md5::Digest;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{jfs::RevisionState, Path};

/// Allocation request.
#[serde_as]
#[derive(Debug, Serialize)]
pub struct AllocReq<'a> {
    /// Path of the file to be uploaded.
    pub path: &'a Path,

    /// How many *more* bytes to allocate.
    pub bytes: u64,

    /// [MD5](https://en.wikipedia.org/wiki/MD5) checksum. For some reason, Jottacloud seems to deduplicate files.
    #[serde(with = "crate::serde::md5_hex")]
    pub md5: md5::Digest,

    /// Handle conflicts.
    pub conflict_handler: ConflictHandler,

    /// Creation date of the file.
    #[serde_as(as = "Option<serde_with::TimestampMilliSeconds<i64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,

    /// Modification date of the file to be uploaded.
    #[serde_as(as = "Option<serde_with::TimestampMilliSeconds<i64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
}

/// Handle conflicts when allocating/uploading a file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConflictHandler {
    /// Reject any conflicts.
    RejectConflicts,
    /// Create a new revision if the file already exists.
    CreateNewRevision,
}

/// Allocation response.
#[derive(Debug, Deserialize)]
pub struct AllocRes {
    /// Name of the file.
    pub name: String,

    /// Path.
    pub path: Path,

    /// State of the file upload (and revision).
    pub state: RevisionState,

    /// Id of the upload. Might be a JWT.
    pub upload_id: String,

    /// Upload url. I think you still need your bearer token to upload.
    pub upload_url: String,

    /// Total number of bytes to upload.
    pub bytes: u64,

    /// Where to resume the upload from, if the upload is chunked for instance.
    pub resume_pos: u64,
}

/// Successful upload response.
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct CompleteUploadRes {
    /// MD5 sum of the upload. If it doesn't match the one specified
    /// in the allocation request, the revision will probably be considered
    /// corrupt by Jottacloud.
    #[serde(with = "crate::serde::md5_hex")]
    pub md5: Digest,

    /// Bytes uploaded in total.
    pub bytes: u64,

    /// Content id?
    pub content_id: String,

    /// Path.
    pub path: Path,

    /// Modification date.
    #[serde_as(as = "serde_with::TimestampMilliSeconds<i64>")]
    pub modified: DateTime<Utc>,
}

/// Pretty-print of the Jottacloud exception returned when performing a
/// chunked upload.
#[derive(Debug)]
pub struct IncompleteUploadRes {
    /// Range of the bytes uploaded now -- NOT the total bytes uploaded (for all chunks).
    pub range: RangeInclusive<u64>,
}

/// Upload response.
#[derive(Debug)]
pub enum UploadRes {
    /// Complete upload.
    Complete(CompleteUploadRes),
    /// Incomplete upload.
    Incomplete(IncompleteUploadRes),
}
