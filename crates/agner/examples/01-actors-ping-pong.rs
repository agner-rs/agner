use std::time::Instant;

use agner::actors::Exit;
use rand::prelude::Distribution;

mod common;

fn main() {
    let multi_thread = std::env::var("MULTI_THREAD").ok().filter(|s| s == "1").is_some();
    let players_count: usize =
        std::env::var("PLAYERS_COUNT").ok().map(|s| s.parse().unwrap()).unwrap_or(16);

    let t0 = Instant::now();

    common::run(multi_thread, async {
        let tokio_init_time = t0.elapsed();

        let t1 = Instant::now();
        let system = common::system(players_count);
        let agner_system_init_time = t1.elapsed();

        let rand_dist = rand::distributions::Uniform::new_inclusive(0.0, 1.0);
        let mut players = vec![];
        let mut name_gen =
            names::Generator::new(names::ADJECTIVES, names::NOUNS, names::Name::Plain);

        let t2 = Instant::now();
        for _ in 0..players_count {
            let name = name_gen.next().unwrap();
            let return_rate = rand_dist.sample(&mut rand::thread_rng());

            let player = system
                .spawn(player::player, (name.to_owned(), return_rate), Default::default())
                .await
                .expect("Failed to start an actor");
            log::info!(
                "Adding player {} [return-rate: {}, actor-id: {}]",
                name,
                return_rate,
                player
            );
            players.push(player);
        }
        let actors_init_time = t2.elapsed();

        let t3 = Instant::now();
        let winner_opt = tournament::run_tournament(&system, players).await;
        let tournament_time = t3.elapsed();

        if let Some(winner) = winner_opt {
            let winner_name = player::api::get_player_name(&system, winner).await;
            log::info!("Congratulations to the winner — {}", winner_name);
            system.exit(winner, Exit::shutdown()).await;
            system.wait(winner).await;
        } else {
            log::info!("No winner today :(");
        }

        eprintln!("TOKIO INIT TIME: {:?}", tokio_init_time);
        eprintln!("AGNER SYS INIT TIME: {:?}", agner_system_init_time);
        eprintln!("ACTORS INIT TIME: {:?}", actors_init_time);
        eprintln!("TOURNAMENT TIME: {:?}", tournament_time);
    });

    eprintln!("TOTAL TIME: {:?}", t0.elapsed());
}

mod tournament {
    //! A module with the tournament logic.

    use crate::player;
    use agner::actors::{ActorID, System};
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;

    /// A run multiple tours until there remains at most one player.
    ///
    /// Each tour reduces the number of participants: the losers leave, the winners — remain.
    pub async fn run_tournament(system: &System, mut players: Vec<ActorID>) -> Option<ActorID> {
        let mut tour_id = 0;
        loop {
            if players.len() <= 1 {
                break players.pop()
            }
            tour_id += 1;
            run_tour(&system, tour_id, &mut players).await;
        }
    }

    // Runs a single tour.
    //
    // The players are popped in pairs out of `players`.
    // If the are odd number of players — the last one gets into the next tour automatically.
    // The matches between the pairs are held simultaneously.
    async fn run_tour(system: &System, tour_id: usize, players: &mut Vec<ActorID>) {
        log::info!("Starting tour #{} [number of participants: {}]", tour_id, players.len());

        let mut matches = vec![];

        let mut opponent = None;

        while let Some(player) = players.pop() {
            if let Some(opponent) = opponent.take() {
                matches.push((player, opponent));
            } else {
                opponent = Some(player);
            }
        }

        if let Some(odd_one) = opponent.take() {
            log::warn!("{} found no pair. Passes to the next tour", odd_one);
            players.push(odd_one);
        }

        let mut futures_unordered = FuturesUnordered::new();
        for (match_id, (left, right)) in matches.into_iter().enumerate() {
            futures_unordered.push(run_match(&system, tour_id, match_id, left, right));
        }
        while let Some(match_winner) = futures_unordered.next().await {
            players.push(match_winner);
        }

        log::info!("End of tour #{} [number of winners: {}]", tour_id, players.len());
    }

    /// Runs a single match between `server` and `receiver`.
    /// Returns the handle to the winning player.
    async fn run_match(
        system: &System,
        tour_id: usize,
        match_id: usize,
        server: ActorID,
        receiver: ActorID,
    ) -> ActorID {
        let server_name = player::api::get_player_name(system, server).await;
        let receiver_name = player::api::get_player_name(system, receiver).await;

        log::debug!("match {}.{}: {} vs {}", tour_id, match_id, server_name, receiver_name);
        player::api::serve(system, server, receiver).await;

        let winner = tokio::select! {
            _right_lost = system.wait(receiver) => server,
            _left_lost = system.wait(server) => receiver,
        };

        let winner_name = player::api::get_player_name(system, winner).await;
        log::debug!("match {}.{}, winner — {}", tour_id, match_id, winner_name);
        winner
    }
}

mod player {
    //! A module with `player` actor.
    //!
    //! Player is started with the following arguments:
    //! - player-name;
    //! - probability of successful return.
    //!
    //! Player can be in either of two states: [`Idle` or `InGame`](crate::player::State).
    //! When [asked to serve](crate::player::api::serve) it sends a message
    //! [`Hit`](crate::player::Message::Hit) to its opponent, and enters the state `InGame`.
    //!
    //! Upon receiving a [`Hit`](crate::player::Message::Hit) the opponent also enters the state
    //! `InGame` and returns a hit to its opponent.
    //!
    //! While `InGame` when a player receives a [`Hit`](crate::player::Message::Hit), it returns it
    //! back with the probability specified in the argument. If it cannot return the hit — it
    //! notifies the opponent with the [`Win` message](crate::player::Message::Win) exits with the
    //! reason [`Loss`](crate::player::Loss).
    //!
    //! The winner gets back into the state `Idle`.

    use agner::actors::{ActorID, Context, Exit, Never, System};
    use rand::prelude::Distribution;
    use tokio::sync::oneshot;

    pub mod api {
        use super::*;

        pub async fn get_player_name(system: &System, player: ActorID) -> String {
            let (tx, rx) = oneshot::channel();
            system.send(player, Message::GetName(tx)).await;
            rx.await.unwrap()
        }

        pub async fn serve(system: &System, server: ActorID, receiver: ActorID) {
            system.send(server, Message::Serve { receiver }).await;
        }
    }

    #[derive(Debug)]
    pub enum Message {
        Serve { receiver: ActorID },
        Hit { from: ActorID },
        Win { opponent: ActorID },
        GetName(oneshot::Sender<String>),
    }

    #[derive(Debug, thiserror::Error)]
    #[error("{} has lost to {}", my_name, opponent_name)]
    pub struct Loss {
        my_name: String,
        opponent_name: String,
    }

    #[derive(Debug, Clone, Copy)]
    enum State {
        Idle,
        InGame { opponent: ActorID },
    }

    pub async fn player(
        context: &mut Context<Message>,
        (my_name, successful_return_probability): (String, f64),
    ) -> Result<Never, Exit> {
        let mut state = State::Idle;

        let rand_dist = rand::distributions::Uniform::new_inclusive(0.0, 1.0);

        loop {
            let message = context.next_message().await;

            match (state, message) {
                // The player can return its player-name in any state.
                (_, Message::GetName(reply_to)) => {
                    let _ = reply_to.send(my_name.to_owned());
                },

                // The player expects the `Serve` message only when it is `Idle`.
                (State::Idle, Message::Serve { receiver: opponent }) => {
                    // link to the opponent, so that both of actors terminate when the other fails.
                    context.link(opponent).await;
                    state = State::InGame { opponent };
                    context
                        .system()
                        .send(opponent, Message::Hit { from: context.actor_id() })
                        .await;
                },

                // If the player receives `Hit` message when it is `Idle` it unconditionally returns
                // the hit and enters the state `InGame`.
                (State::Idle, Message::Hit { from }) => {
                    state = State::InGame { opponent: from };
                    context.system().send(from, Message::Hit { from: context.actor_id() }).await;
                },

                // If the player receives `Hit` message when it is `InGame` it returns the hit with
                // the probability specified in the argument. If it cannot return
                // the hit — it notifies the opponent about own loss.
                (State::InGame { opponent }, Message::Hit { from }) if from == opponent =>
                    if rand_dist.sample(&mut rand::thread_rng()) < successful_return_probability {
                        // Yeay! Successful return!
                        context
                            .system()
                            .send(opponent, Message::Hit { from: context.actor_id() })
                            .await;
                    } else {
                        // The opponent wins. Good game. Thanks.
                        let opponent_name = api::get_player_name(&context.system(), opponent).await;
                        context.unlink(opponent).await;
                        context
                            .system()
                            .send(opponent, Message::Win { opponent: context.actor_id() })
                            .await;

                        context.exit(Exit::custom(Loss { my_name, opponent_name })).await;
                        unreachable!()
                    },

                // The opponent has lost. We win. Ready for another match.
                (State::InGame { opponent }, Message::Win { opponent: from })
                    if from == opponent =>
                {
                    state = State::Idle;
                },

                // Something went wrong. Bail out.
                (state, unexpected_message) => {
                    context
                        .exit(Exit::from_message(format!(
                            "Unexpected message {:?} while in state {:?}",
                            unexpected_message, state
                        )))
                        .await;
                    unreachable!()
                },
            }
        }
    }
}
