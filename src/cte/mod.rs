//! CTE（陷阱/上下文扩展）层：Context、Event、陷阱入口与派发。

mod context;
mod entry;
mod event;
mod handler;
#[cfg(test)]
mod tests;

pub use context::{CONTEXT_SIZE, Context, DEFAULT_TRAP_EXIT, SContext, TrapInfo};
pub use event::{Event, Handler, SHandler, decode_event, decode_event_s};
pub use handler::{trap_init, trap_init_s};
