use std::time::Duration;

use agner_actor::error::ExitReason;
use agner_actor::task::Done;
use agner_actor::{Actor, Context, SharedSeed, System, TaskID, TaskManager};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Unexpected timer id")]
	UnexpectedTimerID,
}

#[derive(Debug)]
pub enum Message {
	Init(TaskID),
	Job(TaskID),
	Exit(TaskID),
}

struct Init;
impl From<Done<Init>> for Message {
	fn from(tc: Done<Init>) -> Self {
		Message::Init(tc.task_id)
	}
}

struct Job;
impl From<Done<Job>> for Message {
	fn from(tc: Done<Job>) -> Self {
		Message::Job(tc.task_id)
	}
}

struct Exit;
impl From<Done<Exit>> for Message {
	fn from(tc: Done<Exit>) -> Self {
		Message::Exit(tc.task_id)
	}
}

#[derive(Debug)]
pub struct ActorWithTasks {
	timer_id: TaskID,
}

#[async_trait::async_trait]
impl<Sys: System> Actor<Sys> for ActorWithTasks {
	type Seed = SharedSeed<()>;
	type Message = Message;

	async fn init<Ctx: Context<Actor = Self, System = Sys, Seed = Self::Seed>>(
		context: &mut Ctx,
	) -> Result<Self, ExitReason> {
		log::info!("init");
		let task_id = context.tasks().task_start(async {
			tokio::time::sleep(Duration::from_secs(1)).await;
			Init
		});
		Ok(Self { timer_id: task_id })
	}

	async fn handle_message<Ctx: Context<Actor = Self, System = Sys, Seed = Self::Seed>>(
		&mut self,
		context: &mut Ctx,
		message: Self::Message,
	) -> Result<(), ExitReason> {
		match message {
			Message::Init(id) => {
				assert_eq!(id, self.timer_id);
				for i in 1..10 {
					context.tasks().task_start(async move {
						tokio::time::sleep(Duration::from_millis(i * 100)).await;
						Job
					});
				}
				self.timer_id = context.tasks().task_start(async {
					tokio::time::sleep(Duration::from_secs(2)).await;
					Exit
				});

				Ok(())
			},
			Message::Job(job_id) => {
				log::info!("job complete: {:?}", job_id);
				Ok(())
			},
			Message::Exit(id) => {
				assert_eq!(id, self.timer_id);
				context.exit(ExitReason::shutdown()).await;
				unreachable!()
			},
		}
	}
}
