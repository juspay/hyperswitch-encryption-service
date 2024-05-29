mod app;
mod crypto;
mod db;

pub(crate) use {app::*, crypto::*, db::*};

pub type CustomResult<T, E> = error_stack::Result<T, E>;

pub trait SwitchError<T, E> {
    fn switch(self) -> CustomResult<T, E>;
}
