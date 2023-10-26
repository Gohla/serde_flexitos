//! [`Serialize`] implementation for serialization of trait objects.

use serde::ser::{Serialize, SerializeMap, Serializer};

/// Serialize `trait_object` as a single `key`-`trait_object` pair where `key` is the unique identifier for the concrete
/// type of `trait_object`
pub struct SerializeTraitObject<'k, 'o, O: ?Sized> {
  pub key: &'k str,
  pub trait_object: &'o O,
}

impl<'a, O: ?Sized + erased_serde::Serialize + 'a> Serialize for SerializeTraitObject<'_, '_, O> {
  #[inline]
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    /// Wrapper so we can implement [`Serialize`] for `Wrap(O)`.
    #[repr(transparent)]
    struct Wrap<'a, O: ?Sized>(&'a O);
    impl<'a, O: ?Sized + erased_serde::Serialize + 'a> Serialize for Wrap<'a, O> {
      #[inline]
      fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        erased_serde::serialize(self.0, serializer)
      }
    }

    let mut map = serializer.serialize_map(Some(1))?;
    map.serialize_entry(self.key, &Wrap(self.trait_object))?;
    map.end()
  }
}
