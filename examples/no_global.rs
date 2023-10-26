use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use serde::{Deserialize, Serialize, Serializer};
use serde::de::DeserializeSeed;

use serde_flexitos::{Registry, serialize_trait_object};
use serde_flexitos::de::{DeserializeMapWith, DeserializeTraitObject, DeserializeVecWithTraitObject};

// Example trait

pub trait ExampleObj: erased_serde::Serialize + Debug {
  fn key(&self) -> &'static str;
}

// Example trait implementations

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);
impl Foo { const KEY: &'static str = "Foo"; }
impl ExampleObj for Foo { fn key(&self) -> &'static str { Self::KEY } }

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bar(usize);
impl Bar { const KEY: &'static str = "Bar"; }
impl ExampleObj for Bar { fn key(&self) -> &'static str { Self::KEY } }

// Serialize implementation

impl Serialize for dyn ExampleObj {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    serialize_trait_object(serializer, self.key(), self)
  }
}

// Run serialization roundtrips

fn main() -> Result<(), Box<dyn Error>> {
  let mut registry = Registry::<dyn ExampleObj>::new("ExampleObj");
  registry.register(Foo::KEY, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::KEY, |d| Ok(Box::new(erased_serde::deserialize::<Bar>(d)?)));

  let foo = Foo("A".to_string());
  let bar = Bar(0);

  { // `Box<dyn ExampleObj>` serialization roundtrip
    let example: Box<dyn ExampleObj> = Box::new(foo.clone());
    let json = serde_json::to_string(example.as_ref())?;
    println!("`Box<dyn ExampleObj>`   serialized: {}", json);

    let deserialize = DeserializeTraitObject(&registry);
    let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new(&json));
    let roundtrip: Box<dyn ExampleObj> = deserialize.deserialize(&mut deserializer)?;
    println!("`Box<dyn ExampleObj>` deserialized: {:?}", roundtrip);
  }

  { // `Vec<Box<dyn ExampleObj>>` serialization roundtrip
    let examples: Vec<Box<dyn ExampleObj>> = vec![Box::new(foo.clone()), Box::new(bar.clone())];
    let json = serde_json::to_string(&examples)?;
    println!("`Vec<Box<dyn ExampleObj>>`   serialized: {}", json);

    let deserialize = DeserializeVecWithTraitObject(&registry);
    let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new(&json));
    let roundtrip: Vec<Box<dyn ExampleObj>> = deserialize.deserialize(&mut deserializer)?;
    println!("`Vec<Box<dyn ExampleObj>>` deserialized: {:?}", roundtrip);
  }

  { // `HashMap<String, Box<dyn ExampleObj>>` serialization roundtrip
    let mut examples = HashMap::<String, Box<dyn ExampleObj>>::new();
    examples.insert("foo".to_string(), Box::new(foo.clone()));
    examples.insert("bar".to_string(), Box::new(bar.clone()));
    let json = serde_json::to_string(&examples)?;
    println!("`HashMap<String, Box<dyn ExampleObj>>`   serialized: {}", json);

    let deserialize = DeserializeMapWith::trait_object_value(&registry);
    let mut deserializer = serde_json::Deserializer::new(serde_json::de::StrRead::new(&json));
    let roundtrip: HashMap::<String, Box<dyn ExampleObj>> = deserialize.deserialize(&mut deserializer)?;
    println!("`HashMap<String, Box<dyn ExampleObj>>` deserialized: {:?}", roundtrip);
  }

  // This example uses `DeserializeTraitObject`, `DeserializeVecWithTraitObject`, and `DeserializeMapWith`, which
  // implement `DeserializeSeed` instead of `Deserialize`.
  //
  // If you need to deserialize trait objects inside your custom data structures, this will require a lot of extra
  // boilerplate, due to `serde_derive` not deriving `DeserializeSeed` implementations. See
  // https://stackoverflow.com/a/75902605 for an example on how to write these implementations.

  Ok(())
}
