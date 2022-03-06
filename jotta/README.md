# Jottacloud API Discoveries

## Uploading

You need to allocate before uploading:

`POST https://api.jottacloud.com/files/v1/allocate`

The **JSON body** must contain a `md5` field, which makes streaming complicated.

A successful allocation will return an `upload_url` that you can `POST` the data to.

### Chunked uploads

Uploads can be easily chunked by `POST`ing to the previously obtained `upload_url` with the desired chunk
and a `Range` header specifying where in the complete file this chunk is located.

Jottacloud will return an `HTTP 420` error (whatever that is) and an `IncompleteUploadOpenApiException`,
but it does work. Trust me. The next allocation call will have a new `resume_pos` field.

- There doesn't seem to be any minimum for the chunk size. **Tested with 1 byte per request.**
- ~~There is no need to allocate between every chunk (`upload_url` can be reused).~~ (*might be false*)
- `resume_pos` is available at the file metadata endpoint too.
