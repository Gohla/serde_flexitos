use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_flexitos::ser::require_erased_serialize_impl;
#[allow(unused_imports)]
use serde_flexitos::MapRegistry;
use serde_flexitos::{serialize_trait_object, DeserializeFn, GetError, Registry};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Debug;
use std::sync::LazyLock;

// Custom registry implementation

/// [Registry] implementation mapping unique identifiers of type `I` to deserialize functions of trait object type `O`,
/// using a [BTreeMap].
///
/// Multiple registrations for the same identifier are ignored. The first deserialize function registered for that
/// identifier is returned instead.
///
/// Only use this registry implementation when you are sure that you want to ignore multiple registrations for the same
/// identifiers without errors, for example when you know that these registrations are pure duplicates: the exact same
/// deserialize function is registered multiple times for the same identifier.
///
/// Do *not* use this implementation with global static registration mechanisms such as [linkme] or [inventory], as
/// there is no guarantee about the order in which registrations are performed. This could lead to subtle bugs where
/// changing an unrelated part of the program changes the deserialization function!
pub struct FirstMapRegistry<O: ?Sized, I = &'static str> {
  deserialize_fns: BTreeMap<I, DeserializeFn<O>>,
  trait_object_name: &'static str,
}

impl<O: ?Sized, I> FirstMapRegistry<O, I> {
  /// Creates a new registry, using `trait_object_name` as the name of `O` for diagnostic purposes.
  #[inline]
  pub fn new(trait_object_name: &'static str) -> Self {
    Self {
      deserialize_fns: BTreeMap::new(),
      trait_object_name,
    }
  }
}

impl<O: ?Sized, I: Ord> Registry for FirstMapRegistry<O, I> {
  type Identifier = I;
  type TraitObject = O;

  #[inline]
  fn register(&mut self, id: Self::Identifier, deserialize_fn: DeserializeFn<Self::TraitObject>) {
    self.deserialize_fns.entry(id).or_insert(deserialize_fn);
  }

  #[inline]
  fn get_deserialize_fn(&self, id: Self::Identifier) -> Result<&DeserializeFn<Self::TraitObject>, GetError<Self::Identifier>> {
    self.deserialize_fns.get(&id).ok_or_else(|| GetError::NotRegistered { id })
  }

  #[inline]
  fn get_trait_object_name(&self) -> &'static str {
    self.trait_object_name
  }
}


// Example trait

pub trait ExampleObj: erased_serde::Serialize + Debug {
  fn id(&self) -> &'static str;
}

// Example trait implementation

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);
impl Foo {
  const ID: &'static str = "Foo";
}
impl ExampleObj for Foo {
  fn id(&self) -> &'static str { Self::ID }
}

// Registry

static EXAMPLE_OBJ_REGISTRY: LazyLock<FirstMapRegistry<dyn ExampleObj>> = LazyLock::new(|| {
  // Use our custom `FirstMapRegistry` here.
  let mut registry = FirstMapRegistry::<dyn ExampleObj>::new("ExampleObj");
  registry.register(Foo::ID, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  // Register `Foo` again, but this will be ignored by our `FirstMapRegistry` implementation. This is fine because we
  // registered `Foo` again with the same deserialize function.
  registry.register(Foo::ID, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry
});

// (De)serialize implementation

impl<'a> Serialize for dyn ExampleObj + 'a {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    const fn __check_erased_serialize_supertrait<T: ?Sized + ExampleObj>() {
      require_erased_serialize_impl::<T>();
    }
    serialize_trait_object(serializer, self.id(), self)
  }
}
impl<'de> Deserialize<'de> for Box<dyn ExampleObj> {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    EXAMPLE_OBJ_REGISTRY.deserialize_trait_object(deserializer)
  }
}

// Run serialization roundtrips

fn main() -> Result<(), Box<dyn Error>> {
  // `Box<dyn ExampleObj>` serialization roundtrip
  let example: Box<dyn ExampleObj> = Box::new(Foo("A".to_string()));
  let json = serde_json::to_string(&example)?;
  println!("`Box<dyn ExampleObj>`   serialized: {}", json);

  let roundtrip: Box<dyn ExampleObj> = serde_json::from_str(&json)?;
  println!("`Box<dyn ExampleObj>` deserialized: {:?}", roundtrip);

  // If you change `FirstMapRegistry` to `MapRegistry` above, deserialization fails "multiple registrations" error.

  Ok(())
}

