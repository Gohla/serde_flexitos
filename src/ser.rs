//! [`Serialize`] implementation for serialization of trait objects.

use serde::ser::{Serialize, SerializeMap, Serializer};

/// Serialize `trait_object` as a single `id`-`trait_object` pair where `id` is the unique identifier for the concrete
/// type of `trait_object`
pub struct SerializeTraitObject<'o, I, O: ?Sized> {
  pub id: I,
  pub trait_object: &'o O,
}

impl<'a, I, O> Serialize for SerializeTraitObject<'_, I, O> where
  I: Serialize,
  O: ?Sized + erased_serde::Serialize + 'a
{
  #[inline]
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
    S: Serializer
  {
    /// Wrapper so we can implement [`Serialize`] for `Wrap(O)`.
    #[repr(transparent)]
    struct Wrap<'a, O: ?Sized>(&'a O);
    impl<'a, O> Serialize for Wrap<'a, O> where
      O: ?Sized + erased_serde::Serialize + 'a
    {
      #[inline]
      fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        erased_serde::serialize(self.0, serializer)
      }
    }

    let mut map = serializer.serialize_map(Some(1))?;
    map.serialize_entry(&self.id, &Wrap(self.trait_object))?;
    map.end()
  }
}

/// Checks whether [`T`] implements [`erased_serde::Serialize`].
pub const fn require_erased_serialize_impl<T: ?Sized + erased_serde::Serialize>() {}
