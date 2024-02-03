//! [`DeserializeSeed`] and [`Visitor`] impls for deserializing trait objects and collections of trait objects.

use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;

use serde::de::{self, Deserializer, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use crate::{DeserializeFn, Registry};

/// Deserialize [`Box<<R as Registry>::TraitObject>`] from a single id-value pair, using the registry to get deserialize
/// functions for concrete types of the trait object. Implements [`DeserializeSeed`].
#[repr(transparent)]
pub struct DeserializeTraitObject<'r, R>(pub &'r R);

impl<'de, R: Registry> DeserializeSeed<'de> for DeserializeTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = Box<R::TraitObject>;

  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_map(self)
  }
}

impl<'de, R: Registry> Visitor<'de> for DeserializeTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = Box<R::TraitObject>;

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

impl<R> Copy for DeserializeTraitObject<'_, R> {}
impl<R> Clone for DeserializeTraitObject<'_, R> {
  #[inline]
  fn clone(&self) -> Self { *self }
}
impl<'de, R: Registry> Display for DeserializeTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  #[inline]
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.expecting(f) }
}


/// Deserialize [`<R as Registry>::Identifier`] and use it to get its deserialize function from the registry.
#[repr(transparent)]
struct IdToDeserializeFn<'r, R>(&'r R);

impl<'de, R: Registry> DeserializeSeed<'de> for IdToDeserializeFn<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = DeserializeFn<R::TraitObject>;

  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    let id = R::Identifier::deserialize(deserializer)?;
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
    self.0(&mut erased).map_err(de::Error::custom)
  }
}


/// Deserialize [`Vec<Box<<R as Registry>::TraitObject>>`], using the registry to get deserialize functions for concrete
/// types of the trait object. Implements [`DeserializeSeed`].
#[repr(transparent)]
pub struct DeserializeVecWithTraitObject<'r, R>(pub &'r R);

impl<'de, R: Registry> DeserializeSeed<'de> for DeserializeVecWithTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = Vec<Box<R::TraitObject>>;

  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_seq(self)
  }
}

impl<'de, R: Registry> Visitor<'de> for DeserializeVecWithTraitObject<'_, R> where
  R::Identifier: Deserialize<'de> + Debug,
{
  type Value = Vec<Box<R::TraitObject>>;

  #[inline]
  fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
    formatter.write_str("a sequence of '")?;
    DeserializeTraitObject(self.0).expecting(formatter)?;
    formatter.write_str("'")
  }

  #[inline]
  fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
    let mut vec = if let Some(capacity) = seq.size_hint() {
      Vec::with_capacity(capacity)
    } else {
      Vec::new()
    };
    while let Some(trait_object) = seq.next_element_seed(DeserializeTraitObject(self.0))? {
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

impl<'k, K, V, R> DeserializeMapWith<DeserializeTraitObject<'k, R>, PhantomData<V>> where
  K: Eq + Hash + ?Sized,
  R: Registry<TraitObject=K>
{
  /// Deserialize `HashMap<Box<K>, V>`, deserializing `Box<K>` as a trait object where `K` is the trait object type,
  /// using `registry` to get deserialize functions for concrete types of trait object `K`.
  #[inline]
  pub fn trait_object_key(registry: &'k R) -> Self {
    Self {
      key_deserialize_seed: DeserializeTraitObject(registry),
      value_deserialize_seed: PhantomData::default(),
    }
  }
}

impl<'v, K, V, R> DeserializeMapWith<PhantomData<K>, DeserializeTraitObject<'v, R>> where
  K: Eq + Hash,
  V: ?Sized,
  R: Registry<TraitObject=V>
{
  /// Deserialize `HashMap<K, Box<V>>`, deserializing `Box<V>` as a trait object where `V` is the trait object type,
  /// using `registry` to get deserialize functions for concrete types of trait object `V`.
  #[inline]
  pub fn trait_object_value(registry: &'v R) -> Self {
    Self {
      key_deserialize_seed: PhantomData::default(),
      value_deserialize_seed: DeserializeTraitObject(registry),
    }
  }
}

impl<'k, 'v, K, RK, V, RV> DeserializeMapWith<DeserializeTraitObject<'k, RK>, DeserializeTraitObject<'v, RV>> where
  K: Eq + Hash + ?Sized,
  V: ?Sized,
  RK: Registry<TraitObject=K>,
  RV: Registry<TraitObject=V>
{
  /// Deserialize `HashMap<Box<K>, Box<V>>`:
  /// - deserialize `Box<K>` as a trait object where `K` is the trait object type, using `key_registry` to get
  ///   deserialize functions for concrete types of trait object `K`.
  /// - deserialize `Box<V>` as a trait object where `V` is the trait object type, using `value_registry` to get
  ///   deserialize functions for concrete types of trait object `V`.
  #[inline]
  pub fn trait_object_key_and_value(key_registry: &'k RK, value_registry: &'v RV) -> Self {
    Self {
      key_deserialize_seed: DeserializeTraitObject(key_registry),
      value_deserialize_seed: DeserializeTraitObject(value_registry),
    }
  }
}

impl<'de, K, V> DeserializeSeed<'de> for DeserializeMapWith<K, V> where
  K: DeserializeSeed<'de> + Copy,
  K::Value: Eq + Hash,
  V: DeserializeSeed<'de> + Copy,
{
  type Value = HashMap<K::Value, V::Value>;

  #[inline]
  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_map(self)
  }
}

impl<'de, K, V> Visitor<'de> for DeserializeMapWith<K, V> where
  K: DeserializeSeed<'de> + Copy,
  K::Value: Eq + Hash,
  V: DeserializeSeed<'de> + Copy,
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
    while let Some(key) = map_access.next_key_seed(self.key_deserialize_seed)? {
      let value = map_access.next_value_seed(self.value_deserialize_seed)?;
      map.insert(key, value);
    }
    Ok(map)
  }
}
