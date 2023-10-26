use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use serde_flexitos::{deserialize_trait_object, Registry, serialize_trait_object};

// Example trait

pub trait ExampleObj<T>: erased_serde::Serialize + Debug {
  fn key(&self) -> &'static str;
  fn get(&self) -> T; // Get some inner value typed by a generic, for example purposes.
}

// Example trait implementations

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);
impl Foo {
  const STRING_KEY: &'static str = "Foo+String";
  const USIZE_KEY: &'static str = "Foo+usize";
}
impl ExampleObj<String> for Foo {
  fn key(&self) -> &'static str { Self::STRING_KEY }
  fn get(&self) -> String { self.0.clone() } // Just gets its value as the generic type
}
impl ExampleObj<usize> for Foo {
  fn key(&self) -> &'static str { Self::USIZE_KEY }
  fn get(&self) -> usize { self.0.len() }
}

// Actually stores something of the generic type.
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bar<T>(T);
impl Bar<String> {
  const KEY: &'static str = "Bar+String";
}
impl Bar<usize> {
  const KEY: &'static str = "Bar+usize";
}
impl ExampleObj<String> for Bar<String> {
  fn key(&self) -> &'static str { Self::KEY }
  fn get(&self) -> String { self.0.clone() }
}
impl ExampleObj<usize> for Bar<usize> {
  fn key(&self) -> &'static str { Self::KEY }
  fn get(&self) -> usize { self.0 }
}

// Registries

static EXAMPLE_OBJ_STRING_REGISTRY: Lazy<Registry<dyn ExampleObj<String>>> = Lazy::new(|| {
  let mut registry = Registry::<dyn ExampleObj<String>>::new("ExampleObj<String>");
  registry.register(Foo::STRING_KEY, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::<String>::KEY, |d| Ok(Box::new(erased_serde::deserialize::<Bar<String>>(d)?)));
  registry
});

static EXAMPLE_OBJ_USIZE_REGISTRY: Lazy<Registry<dyn ExampleObj<usize>>> = Lazy::new(|| {
  let mut registry = Registry::<dyn ExampleObj<usize>>::new("ExampleObj<usize>");
  registry.register(Foo::USIZE_KEY, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::<usize>::KEY, |d| Ok(Box::new(erased_serde::deserialize::<Bar<usize>>(d)?)));
  registry
});

// (De)serialize implementations

impl<T> Serialize for dyn ExampleObj<T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    serialize_trait_object(serializer, self.key(), self)
  }
}

impl<'de> Deserialize<'de> for Box<dyn ExampleObj<String>> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
    deserialize_trait_object(deserializer, &EXAMPLE_OBJ_STRING_REGISTRY)
  }
}

impl<'de> Deserialize<'de> for Box<dyn ExampleObj<usize>> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
    deserialize_trait_object(deserializer, &EXAMPLE_OBJ_USIZE_REGISTRY)
  }
}

// Run serialization roundtrips

fn main() -> Result<(), Box<dyn Error>> {
  let foo = Foo("A".to_string());
  let bar_usize = Bar(0);
  let bar_string = Bar("B".to_string());

  { // `Vec<Box<dyn ExampleObj<usize>>>` serialization roundtrip
    let examples: Vec<Box<dyn ExampleObj<usize>>> = vec![Box::new(foo.clone()), Box::new(bar_usize.clone())];
    let json = serde_json::to_string(&examples)?;
    println!("`Vec<Box<dyn ExampleObj<usize>>>`   serialized: {}", json);

    let roundtrip: Vec<Box<dyn ExampleObj<usize>>> = serde_json::from_str(&json)?;
    println!("`Vec<Box<dyn ExampleObj<usize>>>` deserialized: {:?}", roundtrip);
  }

  { // `HashMap<String, Box<dyn ExampleObj<String>>>` serialization roundtrip
    let mut examples = HashMap::<String, Box<dyn ExampleObj<String>>>::new();
    examples.insert("foo".to_string(), Box::new(foo.clone()));
    examples.insert("bar".to_string(), Box::new(bar_string.clone()));
    let json = serde_json::to_string(&examples)?;
    println!("`HashMap<String, Box<dyn ExampleObj<String>>>`   serialized: {}", json);

    let roundtrip: HashMap::<String, Box<dyn ExampleObj<String>>> = serde_json::from_str(&json)?;
    println!("`HashMap<String, Box<dyn ExampleObj<String>>>` deserialized: {:?}", roundtrip);
  }

  Ok(())
}
