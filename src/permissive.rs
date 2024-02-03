//! [`DeserializeSeed`] and [`Visitor`] implementations for permissive deserialization of trait objects and collections
//! of trait objects. Instead of returning an error, permissive deserialization returns `None` or skips adding a trait
//! object to a collection, when no deserialize function is registered for a concrete type. WIP!

use std::fmt::{self, Debug, Display, Formatter};

use serde::de::{self, Deserializer, DeserializeSeed, MapAccess, Visitor};
use serde::Deserialize;

use crate::{DeserializeFn, GetError, Registry};
use crate::de::DeserializeWithFn;

/// Deserialize [`Option<Box<<R as Registry>::TraitObject>>`] from a single id-value pair, using the registry to get
/// deserialize functions for concrete types of the trait object.  Returns `None` if no deserialize function was found.
/// Implements [`DeserializeSeed`].
#[repr(transparent)]
pub struct PermissiveDeserializeTraitObject<'a, R>(pub &'a R);

impl<'de, R: Registry> DeserializeSeed<'de> for PermissiveDeserializeTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = Option<Box<R::TraitObject>>;

  #[inline]
  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
    deserializer.deserialize_map(self)
  }
}

impl<'de, R: Registry> Visitor<'de> for PermissiveDeserializeTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = Option<Box<R::TraitObject>>;

  #[inline]
  fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
    write!(formatter, "an id-value pair for `Option<Box<dyn {}>>`", self.0.get_trait_object_name())
  }

  #[inline]
  fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
    let Some(deserialize_fn) = map.next_key_seed(PermissiveIdToDeserializeFn(self.0))? else {
      return Err(de::Error::custom(&self));
    };
    let value = if let Some(deserialize_fn) = deserialize_fn {
      Some(map.next_value_seed(DeserializeWithFn(deserialize_fn))?)
    } else {
      None
    };
    Ok(value)
  }
}

impl<R> Copy for PermissiveDeserializeTraitObject<'_, R> {}
impl<R> Clone for PermissiveDeserializeTraitObject<'_, R> {
  #[inline]
  fn clone(&self) -> Self { *self }
}
impl<'de, R: Registry> Display for PermissiveDeserializeTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  #[inline]
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.expecting(f) }
}


/// Deserialize [`<R as Registry>::Identifier`] and use it to get its deserialize function from the registry, returning
/// `None` if no deserialize function was registered.
#[repr(transparent)]
struct PermissiveIdToDeserializeFn<'r, R>(&'r R);

impl<'de, R: Registry> DeserializeSeed<'de> for PermissiveIdToDeserializeFn<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = Option<DeserializeFn<R::TraitObject>>;

  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    let id = R::Identifier::deserialize(deserializer)?;
    match self.0.get_deserialize_fn(id).copied() {
      Ok(v) => Ok(Some(v)),
      Err(GetError::NotRegistered { .. }) => Ok(None),
      Err(e) => Err(de::Error::custom(e)),
    }
  }
}
