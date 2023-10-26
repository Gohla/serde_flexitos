//! [`DeserializeSeed`] and [`Visitor`] impls for deserializing trait objects and collections of trait objects.

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::de::{self, Deserializer, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use crate::{DeserializeFn, Registry};

/// Deserialize `Box<O>` from a single id-value pair, where `O` is the trait object type. Uses given registry to get
/// deserialize functions for concrete types of trait object `O`. Implements [`DeserializeSeed`].
#[repr(transparent)]
pub struct DeserializeTraitObject<'a, O: ?Sized>(pub &'a Registry<O>);

impl<'de, O: ?Sized> DeserializeSeed<'de> for DeserializeTraitObject<'_, O> {
  type Value = Box<O>;
  #[inline]
  fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
    deserializer.deserialize_map(self)
  }
}

impl<'de, O: ?Sized> Visitor<'de> for DeserializeTraitObject<'_, O> {
  type Value = Box<O>;
  #[inline]
  fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
    write!(formatter, "an id-value pair for `Box<dyn {}>`", self.0.get_trait_object_name())
  }
  #[inline]
  fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
    // Visit a single id-value pair. Use `IdToDeserializeFn` to deserialize the ID as a string and then visit it,
    // turning it into `deserialize_fn`.
    let Some(deserialize_fn) = map.next_key_seed(IdToDeserializeFn(self.0))? else {
      return Err(de::Error::custom(&self));
    };
    // Use `DeserializeWithFn` to deserialize the value using `deserialize_fn`, resulting in a deserialized value
    // of trait object `O` (or an error).
    map.next_value_seed(DeserializeWithFn(deserialize_fn))
  }
}

impl<O: ?Sized> Display for DeserializeTraitObject<'_, O> {
  #[inline]
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.expecting(f) }
}

impl<O: ?Sized> Copy for DeserializeTraitObject<'_, O> {}
impl<O: ?Sized> Clone for DeserializeTraitObject<'_, O> {
  #[inline]
  fn clone(&self) -> Self { *self }
}

/// Deserialize ID as string, then visit that string to get the deserialize function from given registry.
#[repr(transparent)]
struct IdToDeserializeFn<'a, O: ?Sized>(&'a Registry<O>);
impl<'de, O: ?Sized> DeserializeSeed<'de> for IdToDeserializeFn<'_, O> {
  type Value = DeserializeFn<O>;
  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_str(self)
  }
}
impl<'de, O: ?Sized> Visitor<'de> for IdToDeserializeFn<'_, O> {
  type Value = DeserializeFn<O>;
  #[inline]
  fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
    write!(formatter, "an id for a deserialize function of `Box<dyn {}>`", self.0.get_trait_object_name())
  }
  #[inline]
  fn visit_str<E: de::Error>(self, id: &str) -> Result<Self::Value, E> {
    self.0.get_deserialize_fn(id).copied().map_err(|e| de::Error::custom(e))
  }
}

/// Deserialize as `Box<O>` using given [deserialize function](DeserializeFn).
#[repr(transparent)]
pub(crate) struct DeserializeWithFn<O: ?Sized>(pub DeserializeFn<O>);
impl<'de, O: ?Sized> DeserializeSeed<'de> for DeserializeWithFn<O> {
  type Value = Box<O>;
  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
    (self.0)(&mut erased).map_err(de::Error::custom)
  }
}


/// Deserialize `Vec<Box<O>>`, deserializing `Box<O>` as a trait object where `O` is the trait object type, using given 
/// registry to get deserialize functions for concrete types of trait object `O`. Implements [`DeserializeSeed`].
#[repr(transparent)]
pub struct DeserializeVecWithTraitObject<'a, O: ?Sized>(pub &'a Registry<O>);

impl<'de, O: ?Sized> DeserializeSeed<'de> for DeserializeVecWithTraitObject<'_, O> {
  type Value = Vec<Box<O>>;
  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_seq(self)
  }
}

impl<'de, O: ?Sized> Visitor<'de> for DeserializeVecWithTraitObject<'_, O> {
  type Value = Vec<Box<O>>;
  #[inline]
  fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
    formatter.write_str("a sequence of '")?;
    DeserializeTraitObject(&self.0).expecting(formatter)?;
    formatter.write_str("'")
  }
  #[inline]
  fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
    let mut vec = if let Some(capacity) = seq.size_hint() {
      Vec::with_capacity(capacity)
    } else {
      Vec::new()
    };
    while let Some(trait_object) = seq.next_element_seed(DeserializeTraitObject(&self.0))? {
      vec.push(trait_object);
    }
    Ok(vec)
  }
}


/// Deserialize `HashMap<K, V>`, using `key_deserialize_seed` to deserialize `K`, and `value_deserialize_seed` to 
/// deserialize `V`. Implements [`DeserializeSeed`]. Use the following functions to create instances of this struct:
/// - [trait_object_key](Self::trait_object_key): deserialize map keys as trait objects,
/// - [trait_object_value](Self::trait_object_value): deserialize map values as trait objects,
/// - [trait_object_key_and_value](Self::trait_object_key_and_value): deserialize map keys and values as trait objects.
pub struct DeserializeMapWith<K, V> {
  key_deserialize_seed: K,
  value_deserialize_seed: V,
}

impl<'a, K: ?Sized + Eq + Hash, V> DeserializeMapWith<DeserializeTraitObject<'a, K>, PhantomData<V>> {
  /// Deserialize `HashMap<Box<K>, V>`, deserializing `Box<K>` as a trait object where `K` is the trait object type, 
  /// using `registry` to get deserialize functions for concrete types of trait object `K`.
  #[inline]
  pub fn trait_object_key(registry: &'a Registry<K>) -> Self {
    Self {
      key_deserialize_seed: DeserializeTraitObject(registry),
      value_deserialize_seed: PhantomData::default(),
    }
  }
}

impl<'a, K: Eq + Hash, V: ?Sized> DeserializeMapWith<PhantomData<K>, DeserializeTraitObject<'a, V>> {
  /// Deserialize `HashMap<K, Box<V>>`, deserializing `Box<V>` as a trait object where `V` is the trait object type, 
  /// using `registry` to get deserialize functions for concrete types of trait object `V`.
  #[inline]
  pub fn trait_object_value(registry: &'a Registry<V>) -> Self {
    Self {
      key_deserialize_seed: PhantomData::default(),
      value_deserialize_seed: DeserializeTraitObject(registry),
    }
  }
}

impl<'k, 'v, K: ?Sized + Eq + Hash, V: ?Sized> DeserializeMapWith<DeserializeTraitObject<'k, K>, DeserializeTraitObject<'v, V>> {
  /// Deserialize `HashMap<Box<K>, Box<V>>`:
  /// - deserialize `Box<K>` as a trait object where `K` is the trait object type, using `key_registry` to get 
  ///   deserialize functions for concrete types of trait object `K`.
  /// - deserialize `Box<V>` as a trait object where `V` is the trait object type, using `value_registry` to get 
  ///   deserialize functions for concrete types of trait object `V`.
  #[inline]
  pub fn trait_object_key_and_value(key_registry: &'k Registry<K>, value_registry: &'v Registry<V>) -> Self {
    Self {
      key_deserialize_seed: DeserializeTraitObject(key_registry),
      value_deserialize_seed: DeserializeTraitObject(value_registry),
    }
  }
}

impl<'de, K, V> DeserializeSeed<'de> for DeserializeMapWith<K, V> where
  K: DeserializeSeed<'de> + Clone,
  K::Value: Eq + Hash,
  V: DeserializeSeed<'de> + Clone,
{
  type Value = HashMap<K::Value, V::Value>;
  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_map(self)
  }
}

impl<'de, K, V> Visitor<'de> for DeserializeMapWith<K, V> where
  K: DeserializeSeed<'de> + Clone,
  K::Value: Eq + Hash,
  V: DeserializeSeed<'de> + Clone,
{
  type Value = HashMap<K::Value, V::Value>;
  #[inline]
  fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
    write!(formatter, "a map with custom key and value `DeserializeSeed` impls")
  }
  #[inline]
  fn visit_map<A: MapAccess<'de>>(self, mut map_access: A) -> Result<Self::Value, A::Error> {
    let mut map = if let Some(capacity) = map_access.size_hint() {
      HashMap::with_capacity(capacity)
    } else {
      HashMap::new()
    };
    while let Some(key) = map_access.next_key_seed(self.key_deserialize_seed.clone())? {
      let value = map_access.next_value_seed(self.value_deserialize_seed.clone())?;
      map.insert(key, value);
    }
    Ok(map)
  }
}
