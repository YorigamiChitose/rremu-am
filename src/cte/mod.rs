//! CTE（陷阱/上下文扩展）层：Context、Event、陷阱入口与派发。

mod context;
mod entry;
mod event;
mod handler;
#[cfg(test)]
mod tests;

pub use context::{Context, SContext, TrapInfo, CONTEXT_SIZE, DEFAULT_TRAP_EXIT};
pub use event::{decode_event, decode_event_s, Event, Handler, SHandler};
pub use handler::{trap_init, trap_init_s};
