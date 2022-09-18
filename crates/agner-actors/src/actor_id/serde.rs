use crate::actor_id::ActorID;

impl serde::Serialize for ActorID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ActorID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let actor_id = String::deserialize(deserializer)?
            .parse()
            .map_err(<<D as serde::Deserializer<'de>>::Error as serde::de::Error>::custom)?;
        Ok(actor_id)
    }
}
