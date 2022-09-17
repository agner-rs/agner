use crate::actor_id::ActorID;

impl serde::Serialize for ActorID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        [self.system(), self.actor(), self.seq()].serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ActorID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let [system, actor, seq] = <[usize; 3]>::deserialize(deserializer)?;
        Ok(Self::new(system, actor, seq))
    }
}
