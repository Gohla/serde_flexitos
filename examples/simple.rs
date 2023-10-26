use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::DeserializeOwned;

use serde_flexitos::{deserialize_trait_object, Registry, serialize_trait_object};

// Example traits

/// Just an example trait, which can be (de)serialized, identified, and debug formatted.
pub trait Example: Serialize + DeserializeOwned + Debug {
  /// Gets the key that uniquely identifies this type for serialization purposes.
  fn key() -> &'static str;
}

/// Object safe proxy of [`Example`], because [`Serialize`], [`DeserializeOwned`], and [`Example::key`] are not
/// object safe. If your trait is already object safe, you don't need a separate object safe proxy.
pub trait ExampleObj: erased_serde::Serialize + Debug {
  /// Gets the key that uniquely identifies this type for serialization purposes. This is a method instead of a
  /// function because this trait must be object-safe; traits with associated functions are not object-safe.
  fn key_dyn(&self) -> &'static str;
}

/// Implement [`ExampleObj`] for all types that implement [`Example`].
impl<T: Example> ExampleObj for T {
  fn key_dyn(&self) -> &'static str { T::key() }
}

// Example trait implementations

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);
impl Example for Foo {
  fn key() -> &'static str { "Foo" }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bar(usize);
impl Example for Bar {
  fn key() -> &'static str { "Bar" }
}

// Registry

static EXAMPLE_OBJ_REGISTRY: Lazy<Registry<dyn ExampleObj>> = Lazy::new(|| {
  let mut registry = Registry::<dyn ExampleObj>::new("ExampleObj");
  registry.register(Foo::key(), |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::key(), |d| Ok(Box::new(erased_serde::deserialize::<Bar>(d)?)));
  registry
});

// (De)serialize implementations

impl<'a> Serialize for dyn ExampleObj {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    serialize_trait_object(serializer, self.key_dyn(), self)
  }
}

impl<'de> Deserialize<'de> for Box<dyn ExampleObj> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
    deserialize_trait_object(deserializer, &EXAMPLE_OBJ_REGISTRY)
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
