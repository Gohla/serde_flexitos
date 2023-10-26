//! [`DeserializeSeed`] and [`Visitor`] implementations for permissive deserialization of trait objects and collections
//! of trait objects. Instead of returning an error, permissive deserialization returns `None` or skips adding a trait
//! object to a collection, when no deserialize function is registered for a concrete type. WIP!

use std::fmt::{self, Display, Formatter};

use serde::de::{self, Deserializer, DeserializeSeed, MapAccess, Visitor};

use crate::{DeserializeFn, GetError, Registry};
use crate::de::DeserializeWithFn;

/// Deserialize `Option<Box<O>>` from a single id-value pair, where `O` is the trait object type. Uses given registry
/// to get deserialize functions for concrete types of trait object `O`. Returns `None` if no deserialize function was
/// found. Implements [`DeserializeSeed`].
#[repr(transparent)]
pub struct PermissiveDeserializeTraitObject<'a, O: ?Sized>(pub &'a Registry<O>);

impl<'de, O: ?Sized> DeserializeSeed<'de> for PermissiveDeserializeTraitObject<'_, O> {
  type Value = Option<Box<O>>;
  #[inline]
  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
    deserializer.deserialize_map(self)
  }
}

impl<'de, O: ?Sized> Visitor<'de> for PermissiveDeserializeTraitObject<'_, O> {
  type Value = Option<Box<O>>;
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

impl<O: ?Sized> Display for PermissiveDeserializeTraitObject<'_, O> {
  #[inline]
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.expecting(f) }
}

impl<O: ?Sized> Copy for PermissiveDeserializeTraitObject<'_, O> {}
impl<O: ?Sized> Clone for PermissiveDeserializeTraitObject<'_, O> {
  #[inline]
  fn clone(&self) -> Self { Self(&self.0) }
}

/// Deserialize ID as string, then visit that string to get the deserialize function from given registry, returning
/// `None` if no deserialize function was registered.
#[repr(transparent)]
struct PermissiveIdToDeserializeFn<'a, O: ?Sized>(&'a Registry<O>);
impl<'de, O: ?Sized> DeserializeSeed<'de> for PermissiveIdToDeserializeFn<'_, O> {
  type Value = Option<DeserializeFn<O>>;
  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_str(self)
  }
}
impl<'de, O: ?Sized> Visitor<'de> for PermissiveIdToDeserializeFn<'_, O> {
  type Value = Option<DeserializeFn<O>>;
  #[inline]
  fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
    write!(formatter, "an id for an optional deserialize function of `Box<dyn {}>`", self.0.get_trait_object_name())
  }
  #[inline]
  fn visit_str<E: de::Error>(self, key: &str) -> Result<Self::Value, E> {
    match self.0.get_deserialize_fn(key).copied() {
      Ok(v) => Ok(Some(v)),
      Err(GetError::NotRegistered { .. }) => Ok(None),
      Err(e) => Err(de::Error::custom(e)),
    }
  }
}
