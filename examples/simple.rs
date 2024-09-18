use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::sync::LazyLock;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::DeserializeOwned;

use serde_flexitos::{MapRegistry, Registry, serialize_trait_object};
use serde_flexitos::ser::require_erased_serialize_impl;

// Example traits

/// Just an example trait, which can be (de)serialized, identified, and debug formatted.
pub trait Example: Serialize + DeserializeOwned + Debug {
  /// The unique and stable identifier of this type.
  const ID: &'static str;
}

/// Object safe proxy of [`Example`], because [`Serialize`], [`DeserializeOwned`], and [`Example::ID`] are not
/// object safe. If your trait is already object safe, you don't need a separate object safe proxy.
pub trait ExampleObj: erased_serde::Serialize + Debug {
  /// Gets the unique and stable identifier for the concrete type of this value. This is a method instead of a function
  /// because this trait must be object-safe; traits with associated functions are not object-safe.
  fn id(&self) -> &'static str;
}

/// Implement [`ExampleObj`] for all types that implement [`Example`].
impl<T: Example> ExampleObj for T {
  fn id(&self) -> &'static str {
    T::ID
  }
}

// Example trait implementations

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);

impl Example for Foo {
  const ID: &'static str = "Foo";
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bar(usize);

impl Example for Bar {
  const ID: &'static str = "Bar";
}

// Registry

static EXAMPLE_OBJ_REGISTRY: LazyLock<MapRegistry<dyn ExampleObj>> = LazyLock::new(|| {
  let mut registry = MapRegistry::<dyn ExampleObj>::new("ExampleObj");
  registry.register(Foo::ID, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::ID, |d| Ok(Box::new(erased_serde::deserialize::<Bar>(d)?)));
  registry
});

// (De)serialize implementations

impl<'a> Serialize for dyn ExampleObj + 'a {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    // Check that `ExampleObj` has `erased_serde::Serialize` as a supertrait, preventing infinite recursion at runtime.
    const fn __check_erased_serialize_supertrait<T: ?Sized + ExampleObj>() {
      require_erased_serialize_impl::<T>();
    }

    serialize_trait_object(serializer, self.id(), self)
  }
}

impl<'de> Deserialize<'de> for Box<dyn ExampleObj> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
    EXAMPLE_OBJ_REGISTRY.deserialize_trait_object(deserializer)
  }
}

// Run serialization roundtrips

fn main() -> Result<(), Box<dyn Error>> {
  let foo = Foo("A".to_string());
  let bar = Bar(0);

  { // Normal serialization roundtrip
    let json = serde_json::to_string(&foo)?;
    println!("`Foo`   serialized: {}", json);

    let roundtrip: Foo = serde_json::from_str(&json)?;
    println!("`Foo` deserialized: {:?}", roundtrip);
  }

  { // `Box<dyn ExampleObj>` serialization roundtrip
    let example: Box<dyn ExampleObj> = Box::new(foo.clone());
    let json = serde_json::to_string(&example)?;
    println!("`Box<dyn ExampleObj>`   serialized: {}", json);

    let roundtrip: Box<dyn ExampleObj> = serde_json::from_str(&json)?;
    println!("`Box<dyn ExampleObj>` deserialized: {:?}", roundtrip);
  }

  { // `Vec<Box<dyn ExampleObj>>` serialization roundtrip
    let examples: Vec<Box<dyn ExampleObj>> = vec![Box::new(foo.clone()), Box::new(bar.clone())];
    let json = serde_json::to_string(&examples)?;
    println!("`Vec<Box<dyn ExampleObj>>`   serialized: {}", json);

    let roundtrip: Vec<Box<dyn ExampleObj>> = serde_json::from_str(&json)?;
    println!("`Vec<Box<dyn ExampleObj>>` deserialized: {:?}", roundtrip);
  }

  { // `HashMap<String, Box<dyn ExampleObj>>` serialization roundtrip
    let mut examples = HashMap::<String, Box<dyn ExampleObj>>::new();
    examples.insert("foo".to_string(), Box::new(foo.clone()));
    examples.insert("bar".to_string(), Box::new(bar.clone()));
    let json = serde_json::to_string(&examples)?;
    println!("`HashMap<String, Box<dyn ExampleObj>>`   serialized: {}", json);

    let roundtrip: HashMap::<String, Box<dyn ExampleObj>> = serde_json::from_str(&json)?;
    println!("`HashMap<String, Box<dyn ExampleObj>>` deserialized: {:?}", roundtrip);
  }

  Ok(())
}
