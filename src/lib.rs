#![doc = include_str!("../README.md")]

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::de::{Deserializer, DeserializeSeed};
use serde::ser::{Serialize, Serializer};

pub mod ser;
pub mod de;
#[cfg(feature = "id_trait")]
pub mod id;
pub mod permissive;

/// Serialize `trait_object` of type `O` with `serializer`, using `id` as the unique identifier for the concrete type of
/// `trait_object`.
#[inline]
pub fn serialize_trait_object<S, O>(
  serializer: S,
  id: &str,
  trait_object: &O,
) -> Result<S::Ok, S::Error> where
  S: Serializer,
  O: erased_serde::Serialize + ?Sized,
{
  ser::SerializeTraitObject { key: id, trait_object }.serialize(serializer)
}

/// Deserialize a trait object of type `O` with `deserializer`. Uses `registry` to get the deserialize function for the
/// concrete type, based on the deserialized ID.
///
/// # Errors
///
/// Returns an error if 0 or more than 1 deserialize functions were registered for the deserialized ID.
#[inline]
pub fn deserialize_trait_object<'de, D, O>(
  deserializer: D,
  registry: &Registry<O>,
) -> Result<Box<O>, D::Error> where
  D: Deserializer<'de>,
  O: ?Sized,
{
  de::DeserializeTraitObject(registry).deserialize(deserializer)
}


/// A registry, mapping unique identifiers of types (keys) to their deserialize functions, enabling deserialization of
/// trait object type `O`.
pub struct Registry<O: ?Sized> {
  deserialize_fns: BTreeMap<&'static str, Option<DeserializeFn<O>>>,
  trait_object_name: &'static str,
}

/// Type alias for deserialize functions of trait object type `O`.
pub type DeserializeFn<O> = for<'de> fn(&mut dyn erased_serde::Deserializer<'de>) -> Result<Box<O>, erased_serde::Error>;

impl<O: ?Sized> Registry<O> {
  /// Creates a new registry, using `trait_object_name` as a name of `O` for diagnostic purposes.
  #[inline]
  pub fn new(trait_object_name: &'static str) -> Self {
    Self {
      deserialize_fns: BTreeMap::new(),
      trait_object_name,
    }
  }
  /// Register `deserialize_fn` as the deserialize function for `id`. If `id` was already registered before,
  /// [get_deserialize_fn](Self::get_deserialize_fn) will forever return `Err(MultipleRegistrations)` for that `id`.
  #[inline]
  pub fn register(&mut self, id: &'static str, deserialize_fn: DeserializeFn<O>) {
    self.deserialize_fns.entry(id)
      .and_modify(|v| { v.take(); })
      .or_insert_with(|| Some(deserialize_fn));
  }

  /// Gets the deserialize function for `id`.
  ///
  /// # Errors
  ///
  /// - `GetError::NotRegistered { id }` if no deserialize function was registered for `id`.
  /// - `GetError::MultipleRegistrations { id }` if multiple deserialize functions were registered for `id`.
  #[inline]
  pub fn get_deserialize_fn<'a>(&self, id: &'a str) -> Result<&DeserializeFn<O>, GetError<'a>> {
    match self.deserialize_fns.get(id) {
      None => Err(GetError::NotRegistered { id }),
      Some(None) => Err(GetError::MultipleRegistrations { id }),
      Some(Some(deserialize_fn)) => Ok(deserialize_fn),
    }
  }
  /// Gets the trait object name, for diagnostic purposes.
  #[inline]
  pub fn get_trait_object_name(&self) -> &'static str { self.trait_object_name }
}

/// Error while getting deserialize function.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum GetError<'a> {
  /// No deserialize function was registered for `id`.
  NotRegistered { id: &'a str },
  /// Multiple deserialize functions were registered for `id`.
  MultipleRegistrations { id: &'a str },
}

impl Error for GetError<'_> {}

impl Display for GetError<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      GetError::NotRegistered { id } => write!(f, "no deserialize function was registered for key '{}'", id),
      GetError::MultipleRegistrations { id } => write!(f, "multiple deserialize functions were registered for key '{}'", id),
    }
  }
}
