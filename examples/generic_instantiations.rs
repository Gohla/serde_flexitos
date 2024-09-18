use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::sync::LazyLock;

use serde_flexitos::ser::require_erased_serialize_impl;
use serde_flexitos::{serialize_trait_object, MapRegistry, Registry};

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
impl<T> Into<Box<dyn ExampleObj<T>>> for Foo where
  Foo: ExampleObj<T>,
{
  fn into(self) -> Box<dyn ExampleObj<T>> { Box::new(self) }
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
impl<T: 'static> Into<Box<dyn ExampleObj<T>>> for Bar<T> where
  Bar<T>: ExampleObj<T>,
{
  fn into(self) -> Box<dyn ExampleObj<T>> { Box::new(self) }
}

// Registries

static EXAMPLE_OBJ_STRING_REGISTRY: LazyLock<MapRegistry<dyn ExampleObj<String>>> = LazyLock::new(|| {
  let mut registry = MapRegistry::<dyn ExampleObj<String>>::new("ExampleObj<String>");
  registry.register_type::<Foo>(Foo::STRING_KEY);
  registry.register_type::<Bar<String>>(Bar::<String>::KEY);
  registry
});

static EXAMPLE_OBJ_USIZE_REGISTRY: LazyLock<MapRegistry<dyn ExampleObj<usize>>> = LazyLock::new(|| {
  let mut registry = MapRegistry::<dyn ExampleObj<usize>>::new("ExampleObj<usize>");
  registry.register_type::<Foo>(Foo::USIZE_KEY);
  registry.register_type::<Bar<usize>>(Bar::<usize>::KEY);
  registry
});

// (De)serialize implementations

impl<'a, T> Serialize for dyn ExampleObj<T> + 'a {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where
    S: Serializer,
  {
    const fn __check_erased_serialize_supertrait<T, O: ?Sized + ExampleObj<T>>() {
      require_erased_serialize_impl::<O>();
    }
    serialize_trait_object(serializer, self.key(), self)
  }
}

impl<'de> Deserialize<'de> for Box<dyn ExampleObj<String>> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
    D: Deserializer<'de>,
  {
    EXAMPLE_OBJ_STRING_REGISTRY.deserialize_trait_object(deserializer)
  }
}

impl<'de> Deserialize<'de> for Box<dyn ExampleObj<usize>> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
    D: Deserializer<'de>,
  {
    EXAMPLE_OBJ_USIZE_REGISTRY.deserialize_trait_object(deserializer)
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
