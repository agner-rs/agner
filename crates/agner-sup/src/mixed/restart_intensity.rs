use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct RestartIntensity<D> {
    pub max_restarts: usize,
    pub within: D,
}

#[derive(Debug, Clone)]
pub struct RestartStats<I>(VecDeque<I>);

impl<D> RestartIntensity<D> {
    pub fn new(max_restarts: usize, within: D) -> Self {
        Self { max_restarts, within }
    }

    pub fn new_stats(&self) -> RestartStats<<D as DurationToInstant>::Instant>
    where
        D: DurationToInstant,
    {
        RestartStats::new()
    }

    pub fn report_exit<I>(&self, stats: &mut RestartStats<I>, now: I) -> Result<(), ()>
    where
        I: ElapsedSince<Elapsed = D>,
    {
        stats.truncate(&now, &self.within).push(now);
        if stats.len() > self.max_restarts {
            Err(())
        } else {
            Ok(())
        }
    }
}

impl<I> RestartStats<I> {
    fn new() -> Self {
        Self(Default::default())
    }
}

impl<I> RestartStats<I> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn truncate(&mut self, now: &I, within: &<I as ElapsedSince>::Elapsed) -> &mut Self
    where
        I: ElapsedSince,
    {
        while self.0.front().into_iter().all(|past| now.elapsed_since(past) > *within) {
            if self.0.pop_front().is_none() {
                break
            };
        }
        self
    }

    fn push(&mut self, now: I) -> &mut Self
    where
        I: Ord,
    {
        if self.0.back().into_iter().all(|past| *past < now) {
            self.0.push_back(now);
            self
        } else {
            panic!("attempt to insert a value that comes earlier than the last in the queue")
        }
    }
}

pub trait DurationToInstant: Clone {
    type Instant: Clone;
}

pub trait ElapsedSince: Ord + Clone {
    type Elapsed: Ord + Clone;
    fn elapsed_since(&self, past: &Self) -> Self::Elapsed;
}

impl DurationToInstant for usize {
    type Instant = usize;
}
impl DurationToInstant for Duration {
    type Instant = Instant;
}

impl ElapsedSince for usize {
    type Elapsed = usize;
    fn elapsed_since(&self, past: &Self) -> Self::Elapsed {
        self.checked_sub(*past).expect("`past` is greater than `self`")
    }
}

impl ElapsedSince for Instant {
    type Elapsed = Duration;
    fn elapsed_since(&self, past: &Self) -> Self::Elapsed {
        self.duration_since(*past)
    }
}

#[test]
fn basic_test_usizes() {
    let intensity = RestartIntensity::new(3, 10);
    let mut stats = intensity.new_stats();

    assert!(intensity.report_exit(&mut stats, 1).is_ok());
    assert!(intensity.report_exit(&mut stats, 2).is_ok());
    assert!(intensity.report_exit(&mut stats, 3).is_ok());
    assert!(intensity.report_exit(&mut stats, 4).is_err());
}

#[test]
fn basic_test_instant_and_duration() {
    let intensity = RestartIntensity::new(3, Duration::from_secs(10));
    let mut stats = intensity.new_stats();

    std::thread::sleep(Duration::from_secs(1));
    assert!(intensity.report_exit(&mut stats, Instant::now()).is_ok());
    std::thread::sleep(Duration::from_secs(1));
    assert!(intensity.report_exit(&mut stats, Instant::now()).is_ok());
    std::thread::sleep(Duration::from_secs(1));
    assert!(intensity.report_exit(&mut stats, Instant::now()).is_ok());
    std::thread::sleep(Duration::from_secs(1));
    assert!(intensity.report_exit(&mut stats, Instant::now()).is_err());
}
