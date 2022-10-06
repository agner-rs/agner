use std::fmt;
use std::str::FromStr;

use super::ActorID;

const FMT_SEPARATOR: char = '.';

impl fmt::Display for ActorID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{FMT_SEPARATOR}{}{FMT_SEPARATOR}{}", self.system(), self.actor(), self.seq())
    }
}

impl FromStr for ActorID {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut acc = Option::<(usize, Option<(usize, Option<usize>)>)>::None;

        let mut parts = s.split(FMT_SEPARATOR);
        let mut next_part = move || {
            parts
                .next()
                .map(|p| p.parse::<usize>().map_err(|_| "parse-int error"))
                .transpose()
        };

        while let Some(p) = next_part()? {
            match acc {
                None => acc = Some((p, None)),
                Some((system, None)) => acc = Some((system, Some((p, None)))),
                Some((system, Some((actor, None)))) => acc = Some((system, Some((actor, Some(p))))),
                Some((_system, Some((_actor, Some(_seq))))) => Err("extra part")?,
            }
        }

        if let Some((system, Some((actor, Some(seq))))) = acc {
            Ok(Self::new(system, actor, seq))
        } else {
            Err("ActorID should be in form of <usize>.<usize>.<usize>")
        }
    }
}

#[test]
fn actor_id_to_and_from_str() {
    for system in (0..100).chain([usize::MAX]) {
        for actor in (0..100).chain([usize::MAX]) {
            for seq in (0..100).chain([usize::MAX]) {
                let actor_id = ActorID::new(system, actor, seq);
                let as_string = format!("{}", actor_id);
                let parsed: ActorID = as_string.parse().unwrap();

                assert_eq!(parsed, actor_id)
            }
        }
    }
}

#[test]
fn actor_id_extra_part() {
    assert!("0.0.0.0".parse::<ActorID>().is_err());
}

#[test]
fn actor_id_missing_part() {
    assert!("0.0".parse::<ActorID>().is_err());
}
