use agner_sys1::system::SystemOne;

pub fn make_system(max_actors: usize) -> SystemOne {
	let config = agner_sys1::system::SystemOneConfig { max_actors, ..Default::default() };

	SystemOne::create(config)
}
