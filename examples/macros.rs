use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use serde_flexitos::id::{Id, IdObj};

// Macro

#[macro_export]
macro_rules! create_registry {
  ($trait_object:ident, $register_macro:ident) => {
    create_registry!($trait_object, $register_macro, serde_flexitos::id::Ident<'static>, serde_flexitos::type_to_ident);
  };
  ($trait_object:ident, $register_macro:ident, $ident:ty, $($type_to_ident:ident)::*) => {
    paste::paste! {
      create_registry!($trait_object, $register_macro, $ident, $($type_to_ident)::*, [<$trait_object:snake:upper _DESERIALIZE_REGISTRY>], [<$trait_object:snake:upper _DESERIALIZE_REGISTRY_DISTRIBUTED_SLICE>]);
    }
  };
  ($trait_object:ident, $register_macro:ident, $ident:ty, $($type_to_ident:ident)::*, $registry:ident, $distributed_slice:ident) => {
    #[linkme::distributed_slice]
    pub static $distributed_slice: [fn(&mut serde_flexitos::MapRegistry<dyn $trait_object, $ident>)] = [..];

    static $registry: std::sync::LazyLock<serde_flexitos::MapRegistry<dyn $trait_object, $ident>> = std::sync::LazyLock::new(|| {
      let mut registry = serde_flexitos::MapRegistry::<dyn $trait_object, $ident>::new(stringify!($trait_object));
      for registry_fn in $distributed_slice {
        registry_fn(&mut registry);
      }
      registry
    });

    impl<'a> serde::Serialize for dyn $trait_object + 'a {
      #[inline]
      fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        const fn __check_erased_serialize_supertrait<T: ?Sized + $trait_object>() {
          serde_flexitos::ser::require_erased_serialize_impl::<T>();
        }
        serde_flexitos::serialize_trait_object(serializer, <Self as serde_flexitos::id::IdObj<$ident>>::id(self), self)
      }
    }

    impl<'a, 'de> serde::Deserialize<'de> for Box<dyn $trait_object + 'a> {
      #[inline]
      fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde_flexitos::Registry;
        $registry.deserialize_trait_object(deserializer)
      }
    }

    #[macro_export]
    macro_rules! $register_macro {
      ($generic:ident<$arg:ty>) => {
        impl serde_flexitos::id::Id<$ident> for $generic<$arg> {
          const ID: $ident = $($type_to_ident)::*!($generic<$arg>);
        }
        impl Into<Box<dyn $trait_object>> for $generic<$arg> where {
          #[inline]
          fn into(self) -> Box<dyn $trait_object> {
            Box::new(self)
          }
        }

        paste::paste! {
          #[linkme::distributed_slice($distributed_slice)]
          #[inline]
          fn [< __register_ $generic:snake _ $arg:snake >](registry: &mut serde_flexitos::MapRegistry<dyn $trait_object, $ident>) {
            use serde_flexitos::Registry;
            registry.register_id_type::<$generic<$arg>>();
          }
        }
      };
      ($concrete:ty) => {
        impl serde_flexitos::id::Id<$ident> for $concrete {
          const ID: $ident = $($type_to_ident)::*!($concrete);
        }
        impl Into<Box<dyn $trait_object>> for $concrete where {
          #[inline]
          fn into(self) -> Box<dyn $trait_object> {
            Box::new(self)
          }
        }

        paste::paste! {
          #[linkme::distributed_slice($distributed_slice)]
          #[inline]
          fn [< __register_ $concrete:snake >](registry: &mut serde_flexitos::MapRegistry<dyn $trait_object, $ident>) {
            use serde_flexitos::Registry;
            registry.register_id_type::<$concrete>();
          }
        }
      };
    }
  };
}

// Example trait

/// Just an example trait, which can be (de)serialized, identified, and debug formatted.
pub trait Example: Serialize + DeserializeOwned + Id + Debug {}

/// Object safe proxy of [`Example`], because [`Serialize`], [`DeserializeOwned`], and [`Id`] are not object safe. If
/// your trait is already object safe, you don't need a separate object safe proxy.
pub trait ExampleObj: erased_serde::Serialize + IdObj + Debug {}

/// Implement [`ExampleObj`] for all types that implement [`Example`].
impl<T: Example> ExampleObj for T {}

// Create `ExampleObj` registry, implement (de)serialize for `dyn ExampleObj`, and create `register_example!` macro.

create_registry!(ExampleObj, register_example);

// Test implementations

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);
impl Example for Foo {}
register_example!(Foo);

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bar<T>(T);
impl Example for Bar<usize> {}
impl Example for Bar<f32> {}
register_example!(Bar<usize>); // It even works with generic instantiations with a single type argument.
register_example!(Bar<f32>);

// Run serialization roundtrips

fn main() -> Result<(), Box<dyn Error>> {
  let foo = Foo("A".to_string());

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
    let examples: Vec<Box<dyn ExampleObj>> = vec![Box::new(foo.clone()), Box::new(Bar(42))];
    let json = serde_json::to_string(&examples)?;
    println!("`Vec<Box<dyn ExampleObj>>`   serialized: {}", json);

    let roundtrip: Vec<Box<dyn ExampleObj>> = serde_json::from_str(&json)?;
    println!("`Vec<Box<dyn ExampleObj>>` deserialized: {:?}", roundtrip);
  }

  { // `HashMap<String, Box<dyn ExampleObj>>` serialization roundtrip
    let mut examples = HashMap::<String, Box<dyn ExampleObj>>::new();
    examples.insert("foo".to_string(), Box::new(foo.clone()));
    examples.insert("bar with f32".to_string(), Box::new(Bar(42.1337)));
    examples.insert("bar with usize".to_string(), Box::new(Bar(1337)));
    let json = serde_json::to_string(&examples)?;
    println!("`HashMap<String, Box<dyn ExampleObj>>`   serialized: {}", json);

    let roundtrip: HashMap::<String, Box<dyn ExampleObj>> = serde_json::from_str(&json)?;
    println!("`HashMap<String, Box<dyn ExampleObj>>` deserialized: {:?}", roundtrip);
  }

  Ok(())
}
