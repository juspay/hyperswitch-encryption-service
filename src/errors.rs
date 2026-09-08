mod app;
mod crypto;
mod db;

pub(crate) use app::*;
pub(crate) use crypto::*;
pub(crate) use db::*;

pub type CustomResult<T, E> = Result<T, error_stack::Report<E>>;

pub trait SwitchError<T, E> {
    #[track_caller]
    fn switch(self) -> CustomResult<T, E>;
}
