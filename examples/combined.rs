use std::error::Error;
use std::fmt::Debug;

use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use serde_flexitos::{MapRegistry, Registry, serialize_trait_object};
use serde_flexitos::ser::require_erased_serialize_impl;

// Example traits

pub trait Example1Obj: erased_serde::Serialize + Debug {
  fn id(&self) -> &'static str;
}

pub trait Example2Obj: erased_serde::Serialize + Debug {
  fn id(&self) -> &'static str;
}

// Example trait implementations

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);
impl Foo {
  const ID: &'static str = "Foo";
}
impl Example1Obj for Foo {
  fn id(&self) -> &'static str { Self::ID }
}
impl Example2Obj for Foo {
  fn id(&self) -> &'static str { Self::ID }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bar(usize);
impl Bar {
  const ID: &'static str = "Bar";
}
impl Example1Obj for Bar {
  fn id(&self) -> &'static str { Self::ID }
}
impl Example2Obj for Bar {
  fn id(&self) -> &'static str { Self::ID }
}

// Registries

static EXAMPLE_1_OBJ_REGISTRY: Lazy<MapRegistry<dyn Example1Obj>> = Lazy::new(|| {
  let mut registry = MapRegistry::<dyn Example1Obj>::new("Example1Obj");
  registry.register(Foo::ID, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::ID, |d| Ok(Box::new(erased_serde::deserialize::<Bar>(d)?)));
  registry
});

static EXAMPLE_2_OBJ_REGISTRY: Lazy<MapRegistry<dyn Example2Obj>> = Lazy::new(|| {
  let mut registry = MapRegistry::<dyn Example2Obj>::new("Example2Obj");
  registry.register(Foo::ID, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::ID, |d| Ok(Box::new(erased_serde::deserialize::<Bar>(d)?)));
  registry
});

// (De)serialize implementations

impl<'a> Serialize for dyn Example1Obj + 'a {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    const fn __check_erased_serialize_supertrait<T: ?Sized + Example1Obj>() {
      require_erased_serialize_impl::<T>();
    }
    serialize_trait_object(serializer, self.id(), self)
  }
}
impl<'de> Deserialize<'de> for Box<dyn Example1Obj> {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    EXAMPLE_1_OBJ_REGISTRY.deserialize_trait_object(deserializer)
  }
}

impl<'a> Serialize for dyn Example2Obj + 'a {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    const fn __check_erased_serialize_supertrait<T: ?Sized + Example2Obj>() {
      require_erased_serialize_impl::<T>();
    }
    serialize_trait_object(serializer, self.id(), self)
  }
}
impl<'de> Deserialize<'de> for Box<dyn Example2Obj> {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    EXAMPLE_2_OBJ_REGISTRY.deserialize_trait_object(deserializer)
  }
}

// Struct with boxed trait objects, with derived `Serialize` and `Deserialize` implementations.

#[derive(Debug, Serialize, Deserialize)]
struct Combined {
  example_1_obj: Box<dyn Example1Obj>,
  example_2_obj: Box<dyn Example2Obj>,
}
impl Combined {
  pub fn new(example_1_obj: impl Example1Obj + 'static, example_2_obj: impl Example2Obj + 'static) -> Self {
    Self { example_1_obj: Box::new(example_1_obj), example_2_obj: Box::new(example_2_obj) }
  }
}

// Run serialization roundtrips

fn main() -> Result<(), Box<dyn Error>> {
  {
    let json = serde_json::to_string(&Combined::new(Foo("A".to_string()), Bar(0)))?;
    println!("`Combined`   serialized: {}", json);
    let roundtrip: Combined = serde_json::from_str(&json)?;
    println!("`Combined` deserialized: {:?}", roundtrip);
  }

  {
    let json = serde_json::to_string(&Combined::new(Bar(1337), Foo("asd".to_string())))?;
    println!("`Combined`   serialized: {}", json);
    let roundtrip: Combined = serde_json::from_str(&json)?;
    println!("`Combined` deserialized: {:?}", roundtrip);
  }

  Ok(())
}
