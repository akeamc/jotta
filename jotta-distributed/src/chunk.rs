use bytes::Bytes;
use jotta::{
    auth::TokenStore,
    files::{AllocReq, ConflictHandler, UploadRes},
    path::PathOnDevice,
};
use tracing::{instrument, trace};

use crate::Context;

pub type Id = u64;

#[instrument(level = "trace", skip(ctx, body))]
async fn upload_bytes(
    ctx: &Context<impl TokenStore>,
    id: Id,
    body: Bytes, // there is no point accepting a stream since a checksum needs to be calculated prior to allocation anyway
    reject_conflicts: bool,
) -> crate::Result<()> {
    let md5 = md5::compute(&body);
    let size = body.len().try_into().unwrap();

    trace!("uploading {} bytes", size);

    let req = AllocReq {
        path: &ctx.device_root(),
        bytes: size,
        md5,
        conflict_handler: if reject_conflicts {
            ConflictHandler::RejectConflicts
        } else {
            ConflictHandler::CreateNewRevision
        },
        created: None,
        modified: None,
    };

    let upload_url = ctx.client.allocate(&req).await?.upload_url;

    let res = ctx.client.upload_range(&upload_url, body, 0..=size).await?;

    assert!(matches!(res, UploadRes::Complete(_)));

    Ok(())
}
