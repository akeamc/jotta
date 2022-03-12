/// A Jotta object represents a file.
#[derive(Debug)]
pub struct Object {
    /// Size of the object in bytes.
    pub size: u64,
    /// MD5 checksum.
    pub md5: md5::Digest,
}

impl Object {}
