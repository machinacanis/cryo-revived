//! Type-erased handler storage + the [`Handler`] trait (axum-style).
//!
//! A handler is any `Fn(args...) -> Future` where every arg implements
//! [`FromEvent`] and the future's output implements [`IntoOutcome`]
//! (`()` or `Result<(), crate::Error>`).

use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::sync::Arc;

use crate::context::EventCtx;
use crate::extract::FromEvent;
use crate::Result;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Type-erased handler, callable against any event context.
pub(crate) type Erased = Arc<dyn Fn(&EventCtx) -> BoxFuture<'static, Result<()>> + Send + Sync>;

/// What a handler future may produce: `()` for infallible handlers,
/// or `cryo::Result<()>` (i.e. `anyhow::Result<()>`).
///
/// ```text
/// |c: Ctx| async move { println!("hi"); }           // output = ()
/// |c: Ctx| async move { c.reply().text("x").await } // output = Result<()>
/// |c: Ctx| c.say("hi")                              // builders await themselves
/// ```
///
/// The error type is fixed to `cryo::Error` (anyhow) on purpose: a single
/// `Result` impl lets `Ok(())` be inferred, so closures with `?` need no
/// `Ok::<(), cryo::Error>(())` annotation. `?` converts any
/// `std::error::Error` via anyhow — no `From` impl needed.
pub trait IntoOutcome: Send + 'static {
    /// Normalize into the erased result type.
    fn into_outcome(self) -> Result<()>;
}

impl IntoOutcome for () {
    fn into_outcome(self) -> Result<()> {
        Ok(())
    }
}

impl IntoOutcome for Result<()> {
    fn into_outcome(self) -> Result<()> {
        self
    }
}

/// Implemented automatically for closures/fns whose parameters all
/// implement [`FromEvent`] — you never implement this by hand.
pub trait Handler<Args>: Clone + Send + Sync + 'static {
    /// Extract args from `ctx` and run the handler.
    fn call(&self, ctx: EventCtx) -> BoxFuture<'static, Result<()>>;
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
        impl<F, Fut> Handler<()> for F
        where
            F: Fn() -> Fut + Clone + Send + Sync + 'static,
            Fut: IntoFuture,
            Fut::IntoFuture: Future + Send + 'static,
            <Fut::IntoFuture as Future>::Output: IntoOutcome,
        {
            fn call(&self, _ctx: EventCtx) -> BoxFuture<'static, Result<()>> {
                let f = self.clone();
                Box::pin(async move { f().into_future().await.into_outcome() })
            }
        }
    };
    ($($arg:ident : $ty:ident),+) => {
        #[allow(non_snake_case)]
        impl<F, Fut, $($ty,)+> Handler<($($ty,)+)> for F
        where
            F: Fn($($ty),+) -> Fut + Clone + Send + Sync + 'static,
            Fut: IntoFuture,
            Fut::IntoFuture: Future + Send + 'static,
            <Fut::IntoFuture as Future>::Output: IntoOutcome,
            $($ty: FromEvent,)+
        {
            fn call(&self, ctx: EventCtx) -> BoxFuture<'static, Result<()>> {
                let f = self.clone();
                let args = (|| -> Result<($($ty,)+)> {
                    Ok(($($ty::from_event(&ctx)?,)+))
                })();
                match args {
                    Ok(($($arg,)+)) => {
                        Box::pin(async move {
                            f($($arg),+).into_future().await.into_outcome()
                        })
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
