use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use super::ActorID;

#[test]
fn actor_id_basic_props() {
	const SYSTEMS_RANGE: Range<usize> = 0..10;
	const ACTORS_RANGE: Range<usize> = 0..100;
	const SEQ_RANGE: Range<usize> = 0..100;

	let mut components = Vec::new();
	let mut tree = BTreeMap::new();
	let mut hmap = HashMap::new();

	for system in SYSTEMS_RANGE {
		for actor in ACTORS_RANGE {
			for seq in SEQ_RANGE {
				components.push((system, actor, seq));

				let actor_id = ActorID::new(system, actor, seq);
				assert_eq!(actor_id.system(), system);
				assert_eq!(actor_id.actor(), actor);
				assert_eq!(actor_id.seq(), seq);

				tree.insert(actor_id, (system, actor, seq));
				hmap.insert(actor_id, (system, actor, seq));
			}
		}
	}

	assert_eq!(components.len(), tree.len());
	assert_eq!(components.len(), hmap.len());

	for tuple @ (system, actor, seq) in components {
		let actor_id = ActorID::new(system, actor, seq);

		assert_eq!(Some(tuple), tree.remove(&actor_id));
		assert_eq!(Some(tuple), hmap.remove(&actor_id));
	}

	assert_eq!(0, tree.len());
	assert_eq!(0, hmap.len());
}
