use std::fmt;

use tokio::sync::mpsc;

use agner_actor::Actor;

use crate::sys_msg::SysMsg;
use crate::system::SystemOne;

pub(crate) fn create<A: Actor<SystemOne>>(
	serial_id: usize,
	tx_msg: mpsc::UnboundedSender<A::Message>,
	tx_sys: mpsc::UnboundedSender<SysMsg>,
) -> Box<dyn ActorEntry> {
	let tx_msg = Box::new(tx_msg);
	let entry = ActorEntryImpl::<A> { _pd: Default::default(), serial_id, tx_msg, tx_sys };
	Box::new(entry)
}

pub(crate) trait ActorEntry: fmt::Debug + Send + Sync + 'static {
	fn serial_id(&self) -> usize;
	fn tx_msg(&self) -> &dyn std::any::Any;
	fn tx_sys(&self) -> &mpsc::UnboundedSender<SysMsg>;
}

struct ActorEntryImpl<A: Actor<SystemOne>> {
	_pd: std::marker::PhantomData<A>,
	serial_id: usize,
	tx_msg: Box<dyn std::any::Any + Send + Sync>,
	tx_sys: mpsc::UnboundedSender<SysMsg>,
}

impl<A: Actor<SystemOne>> fmt::Debug for ActorEntryImpl<A> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActorEntry")
			.field("type", &std::any::type_name::<A>())
			.field("serial_id", &self.serial_id)
			.finish()
	}
}

impl<A: Actor<SystemOne>> ActorEntry for ActorEntryImpl<A> {
	fn serial_id(&self) -> usize {
		self.serial_id
	}
	fn tx_msg(&self) -> &dyn std::any::Any {
		self.tx_msg.as_ref()
	}
	fn tx_sys(&self) -> &mpsc::UnboundedSender<SysMsg> {
		&self.tx_sys
	}
}
