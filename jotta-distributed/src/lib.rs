mod chunk;

pub(crate) type Result<T> = core::result::Result<T, jotta::Error>;

pub struct Context<S> {
    client: jotta::Client<S>,
}
