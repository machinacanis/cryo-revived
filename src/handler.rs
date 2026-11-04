//! Type-erased handler storage + the [`Handler`] trait (axum-style).
//!
//! A handler is any `Fn(args...) -> Future<Output = Result<(), E>>` where
//! every arg implements [`FromEvent`] and `E: Into<BoxError>`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::context::EventCtx;
use crate::error::BoxError;
use crate::extract::FromEvent;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Type-erased handler, callable against any event context.
pub(crate) type Erased =
    Arc<dyn Fn(&EventCtx) -> BoxFuture<'static, Result<(), BoxError>> + Send + Sync>;

/// Implemented automatically for closures/fns whose parameters all
/// implement [`FromEvent`] — you never implement this by hand.
pub trait Handler<Args>: Clone + Send + Sync + 'static {
    fn call(&self, ctx: EventCtx) -> BoxFuture<'static, Result<(), BoxError>>;
}

pub(crate) fn erase<H, Args>(h: H) -> Erased
where
    H: Handler<Args>,
{
    let h = Arc::new(h);
    Arc::new(move |ctx: &EventCtx| h.call(ctx.clone()))
}

macro_rules! impl_handler {
    () => {
        impl<F, Fut, E> Handler<()> for F
        where
            F: Fn() -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = std::result::Result<(), E>> + Send + 'static,
            E: Into<BoxError>,
        {
            fn call(&self, _ctx: EventCtx) -> BoxFuture<'static, Result<(), BoxError>> {
                let f = self.clone();
                Box::pin(async move { f().await.map_err(Into::into) })
            }
        }
    };
    ($($arg:ident : $ty:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, E, $($ty,)+> Handler<($($ty,)+)> for F
        where
            F: Fn($($ty),+) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = std::result::Result<(), E>> + Send + 'static,
            E: Into<BoxError>,
            $($ty: FromEvent,)+
        {
            fn call(&self, ctx: EventCtx) -> BoxFuture<'static, Result<(), BoxError>> {
                let f = self.clone();
                let args = (|| -> Result<($($ty,)+), BoxError> {
                    Ok(($($ty::from_event(&ctx)?,)+))
                })();
                match args {
                    Ok(($($arg,)+)) => {
                        Box::pin(async move { f($($arg),+).await.map_err(Into::into) })
                    }
                    Err(e) => Box::pin(async move { Err(e) }),
                }
            }
        }
    };
}

impl_handler!();
impl_handler!(t1: T1);
impl_handler!(a1: T1, a2: T2);
impl_handler!(a1: T1, a2: T2, a3: T3);
impl_handler!(a1: T1, a2: T2, a3: T3, a4: T4);
impl_handler!(a1: T1, a2: T2, a3: T3, a4: T4, a5: T5);
impl_handler!(a1: T1, a2: T2, a3: T3, a4: T4, a5: T5, a6: T6);
impl_handler!(a1: T1, a2: T2, a3: T3, a4: T4, a5: T5, a6: T6, a7: T7);
impl_handler!(a1: T1, a2: T2, a3: T3, a4: T4, a5: T5, a6: T6, a7: T7, a8: T8);
