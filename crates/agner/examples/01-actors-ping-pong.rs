use std::time::{Duration, Instant};

use agner::actors::Context;
use agner_actors::{ActorID, Exit, Never, System};
use agner_utils::future_timeout_ext::FutureTimeoutExt;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use rand::prelude::Distribution;
use tokio::sync::oneshot;

mod common;

#[derive(Debug)]
enum Message {
    Serve { opponent: ActorID },
    Inbound { from: ActorID },
    Win { opponent: ActorID },
    GetName(oneshot::Sender<String>),
}

#[derive(Debug, thiserror::Error)]
#[error("{} has lost to {}", my_name, opponent_name)]
struct Loss {
    my_name: String,
    opponent_name: String,
}

#[derive(Debug, Clone, Copy)]
enum State {
    Idle,
    InGame { opponent: ActorID },
}

async fn get_player_name(system: &System, player: ActorID) -> String {
    let (tx, rx) = oneshot::channel();
    system.send(player, Message::GetName(tx)).await;
    rx.await.unwrap()
}

async fn player(
    context: &mut Context<Message>,
    (my_name, return_rate): (String, f64),
) -> Result<Never, Exit> {
    let mut state = State::Idle;

    let rand_dist = rand::distributions::Uniform::new_inclusive(0.0, 1.0);

    loop {
        let message = context.next_message().await;

        match (state, message) {
            (_, Message::GetName(reply_to)) => {
                let _ = reply_to.send(my_name.to_owned());
            },
            (State::Idle, Message::Serve { opponent }) => {
                context.link(opponent).await;
                state = State::InGame { opponent };
                context
                    .system()
                    .send(opponent, Message::Inbound { from: context.actor_id() })
                    .await;
            },
            (State::Idle, Message::Inbound { from }) => {
                state = State::InGame { opponent: from };
                context.system().send(from, Message::Inbound { from: context.actor_id() }).await;
            },
            (State::InGame { opponent }, Message::Win { opponent: from }) if from == opponent => {
                state = State::Idle;
            },
            (State::InGame { opponent }, Message::Inbound { from }) if from == opponent =>
                if rand_dist.sample(&mut rand::thread_rng()) > return_rate {
                    let opponent_name = get_player_name(&context.system(), opponent).await;
                    context.unlink(opponent).await;
                    context
                        .system()
                        .send(opponent, Message::Win { opponent: context.actor_id() })
                        .await;

                    context.exit(Exit::custom(Loss { my_name, opponent_name })).await;
                    unreachable!()
                } else {
                    context
                        .system()
                        .send(opponent, Message::Inbound { from: context.actor_id() })
                        .await;
                },

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
                .spawn(player, (name.to_owned(), return_rate), Default::default())
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
        let mut tour_id = 0;
        let winner = loop {
            if players.len() <= 1 {
                break players.pop()
            }
            tour_id += 1;
            players = run_tour(&system, tour_id, players).await;
        };
        let tournament_time = t3.elapsed();

        if let Some(winner) = winner {
            let winner_name = get_player_name(&system, winner).await;
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

async fn run_tour(system: &System, tour_id: usize, mut players: Vec<ActorID>) -> Vec<ActorID> {
    log::info!("Starting tour #{}", tour_id);

    let mut next_tour = vec![];
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
        next_tour.push(odd_one);
    }

    let mut futures_unordered = FuturesUnordered::new();
    for (match_id, (left, right)) in matches.into_iter().enumerate() {
        futures_unordered.push(run_match(&system, tour_id, match_id, left, right));
    }
    while let Some(match_winner) = futures_unordered.next().await {
        next_tour.extend(match_winner.into_iter());
    }

    log::info!("End of tour #{}", tour_id);

    next_tour
}

async fn run_match(
    system: &System,
    tour_id: usize,
    match_id: usize,
    left: ActorID,
    right: ActorID,
) -> Option<ActorID> {
    log::debug!("match {}.{}: {} vs {}", tour_id, match_id, left, right);
    system.send(left, Message::Serve { opponent: right }).await;

    let winner = tokio::select! {
        _right_lost = system.wait(right) => left,
        _left_lost = system.wait(left) => right,
    };

    if system.wait(winner).timeout(Duration::from_millis(1)).await.is_err() {
        log::debug!("match {}.{}, winner — {}", tour_id, match_id, winner);
        Some(winner)
    } else {
        log::debug!("match {}.{}, tie :(", tour_id, match_id);
        None
    }
}
