//! This crate provides types and function for flexible serialization and deserialization of trait objects with
//! [serde][serde].
//!
//! # Why?
//!
//! When you need to treat several types that implement a trait *as a single type*, a trait object type `dyn O` is one
//! of the most convenient solutions in Rust. If you need (de)serialization for such a trait object, this crate can help
//! provide that.
//!
//! # Is this not already possible?
//!
//! Serializing a trait object is already possible with [erased-serde][erased-serde]'s [`erased_serde::Serialize`].
//! However, erased-serde does not provide a convenient way to *deserialize trait objects*.
//!
//! To deserialize a trait object, we first need to figure out the concrete type that was serialized, and then use the
//! corresponding [`Deserialize`] implementation of that type to deserialize the value. We cannot use the trait object
//! type directly to get the corresponding [`Deserialize`] implementation, because trait objects must be object safe,
//! ruling out associated functions (only allowing methods: functions that take a `&self` and variations). But when
//! deserializing, we do not have an instance of the trait object (we are instantiating it with deserialization!), thus
//! there is no method to call. Therefore, an external mechanism is needed to get [`Deserialize`] implementations for
//! concrete types.
//!
//! Two other solutions exist (to my knowledge), but they make trade-offs that not everyone is willing to make, and
//! provide no way to opt-out of those trade-offs:
//! - [typetag][typetag]: A convenient solution to get (de)serialization for trait objects. However, it uses
//!   [inventory][inventory] to register [`Deserialize`] implementations, which does not work on every platform (for
//!   example, WASM). It also registers these implementations globally using a procedural macro that has to be applied
//!   to every concrete type. Finally, generic traits and generic impls of traits are not supported.
//!   If you can work within these limitations, [typetag][typetag] is a great crate, and you should probably use it
//!   instead of this one because it is more convenient!
//! - [serde_traitobject][serde_traitobject]: An interesting solution that (de)serializes the entire vtable of a trait
//!   object. However, the vtable is not stable across different compilations, changes when you add or remove
//!   implementations of your trait object, and requires nightly Rust. Therefore, this crate is only usable in the very
//!   limited scope of sending trait objects to another process of the same binary, but is very convenient for that
//!   specific use-case.
//!
//! This crate provides types and functions for flexible (de)serialization of trait objects, that do not necessarily
//! require macros, global registration, nightly rust, nor require your binary not to change, at the cost of some
//! convenience. However, convenience can be brought back by creating layers on top of this crate, such as a global
//! registration macro, allowing you to make the trade-off between convenience and flexibility yourself.
//!
//! # How does it work?
//!
//! A trait object is serialized as an id-value pair, also known as the [externally tagged enum representation][exttag],
//! where the id is the *unique identifier* for the concrete type of the value, and the value is serialized using the
//! trait object's [`erased_serde::Serialize`] implementation.
//!
//! A trait object is deserialized by first deserializing the ID, then finding the [`Deserialize`] implementation of
//! the concrete type using that ID, and then deserializing the value with that deserialize impl.
//!
//! An ID must uniquely identify a concrete type of a trait object, and be stable over time, in order for
//! deserialization to keep working over time. Missing or duplicate IDs will result in (recoverable) errors during
//! deserialization.
//!
//! # How do I use this crate?
//!
//! A [registry](Registry) handles registration of [`Deserialize`] impls and finding them by ID. For each trait object
//! you wish to deserialize, you must construct a registry and register all concrete types with it. Currently,
//! [`MapRegistry`] is the only registry implementation.
//!
//! To [register](Registry::register) a concrete type, we must provide:
//! 1) the ID (`&'static str`) for that concrete type,
//! 2) a [deserialize function](DeserializeFn) that deserializes the concrete type as a boxed trait object.
//!
//! Traits must have [`erased_serde::Serialize`] as a supertrait and have a method to retrieve the ID of the concrete
//! type. Concrete types of the trait must implement [`Serialize`].
//!
//! Then, you can implement [`Serialize`] for `dyn Trait` using  [`serialize_trait_object`], and [`Deserialize`] for
//! `Box<dyn Trait>` using [`deserialize_trait_object`](Registry::deserialize_trait_object).
//!
//! # Example
//!
//! An example, using a global registry to get some convenience:
//!
//! ```
//! use once_cell::sync::Lazy;
//! use serde::{Deserialize, Deserializer, Serialize, Serializer};
//!
//! use serde_flexitos::{MapRegistry, Registry, serialize_trait_object};
//!
//! // Trait we want to serialize trait objects of. This example just uses `Debug` as supertrait so we can print values.
//!
//! pub trait Example: erased_serde::Serialize + std::fmt::Debug {
//!   // Gets the ID that uniquely identifies the concrete type of this value. Must be a method for object safety.
//!   fn id(&self) -> &'static str;
//! }
//!
//! // Implementations of the `Example` trait.
//!
//! #[derive(Clone, Serialize, Deserialize, Debug)]
//! struct Foo(String);
//! impl Foo {
//!   const ID: &'static str = "Foo";
//! }
//! impl Example for Foo {
//!   fn id(&self) -> &'static str { Self::ID }
//! }
//!
//! #[derive(Clone, Serialize, Deserialize, Debug)]
//! struct Bar(usize);
//! impl Bar {
//!   const ID: &'static str = "Bar";
//! }
//! impl Example for Bar {
//!   fn id(&self) -> &'static str { Self::ID }
//! }
//!
//! // Create registry for `Example` and register all concrete types with it. Store in static with `Lazy` to lazily
//! // initialize it once while being able to create global references to it.
//!
//! static EXAMPLE_REGISTRY: Lazy<MapRegistry<dyn Example>> = Lazy::new(|| {
//!   let mut registry = MapRegistry::<dyn Example>::new("Example");
//!   registry.register(Foo::ID, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
//!   registry.register(Bar::ID, |d| Ok(Box::new(erased_serde::deserialize::<Bar>(d)?)));
//!   registry
//! });
//!
//! // (De)serialize implementations
//!
//! impl<'a> Serialize for dyn Example + 'a {
//!   fn serialize<S: Serializer >(&self, serializer: S) -> Result<S::Ok, S::Error> {
//!     // Check that `Example` has `erased_serde::Serialize` as a supertrait, preventing infinite recursion at runtime.
//!     const fn __check_erased_serialize_supertrait<T: ?Sized + Example>() {
//!       serde_flexitos::ser::require_erased_serialize_impl::<T>();
//!     }
//!     serialize_trait_object(serializer, self.id(), self)
//!   }
//! }
//!
//! impl<'de> Deserialize<'de> for Box<dyn Example> {
//!   fn deserialize<D: Deserializer<'de> >(deserializer: D) -> Result<Self, D::Error> {
//!     EXAMPLE_REGISTRY.deserialize_trait_object(deserializer)
//!   }
//! }
//!
//! // Run serialization roundtrip
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   let examples: Vec<Box<dyn Example>> = vec![Box::new(Foo("A".to_string())), Box::new(Bar(0))];
//!   println!("Examples: {:?}", examples);
//!   let json = serde_json::to_string(&examples)?;
//!   println!("Serialized: {}", json);
//!   let roundtrip: Vec<Box<dyn Example>> = serde_json::from_str(&json)?;
//!   println!("Deserialized: {:?}", roundtrip);
//!   Ok(())
//! }
//! ```
//!
//! Check out the examples for more use-cases:
//! - `example/simple.rs`: A full version of the above example.
//! - `example/macros.rs`: Convenience macro layered on top of this crate, using [linkme][linkme] to register types.
//! - `example/no_global.rs`: Use a local registry instead of a global one, using [`DeserializeSeed`] implementations
//!   provided by this crate.
//! - `example/generic_instantiations.rs`: Create and use registries for _instantiations_ of generic traits/structs.
//!   Does not handle traits nor structs generically though!
//!
//! # Limitations
//!
//! ## Generic Serialization and Deserialization
//!
//! Serialization and deserialization of trait objects with generic type parameters and structs with generic type
//! parameters is not supported, and I think it is impossible to support this. To support this, we would need a function
//! that goes from a run-time unique identifier (`&str`) to a compile-time type, but that is impossible. This makes
//! sense, because the compiler needs to know at compile-time which (combination of) concrete types are used as generic
//! type arguments to do monomorphization.
//!
//! However, it is possible to register all concrete instances of types that you wish to deserialize, as is done in
//! `example/generic_instantiations.rs`.
//!
//! ## Other Representations
//!
//! Only the [externally tagged enum representation][exttag] is supported for (de)serializing trait objects, to simplify
//! the implementations in this crate. This is only a problem if you need to accept serialized trait objects that were
//! serialized externally using a different representation (i.e., not this crate).
//!
//! # Inspiration
//!
//! This crate is inspired by the excellent [typetag][typetag] crate.
//!
//! [serde]: https://crates.io/crates/serde
//! [erased-serde]: https://crates.io/crates/erased-serde
//! [exttag]: https://serde.rs/enum-representations.html#externally-tagged
//! [typetag]: https://crates.io/crates/typetag
//! [linkme]: https://crates.io/crates/linkme
//! [inventory]: https://crates.io/crates/inventory
//! [objs]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
//! [serde_traitobject]: https://crates.io/crates/serde_traitobject

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

use serde::de::{DeserializeOwned, Deserializer, DeserializeSeed};
use serde::Deserialize;
use serde::ser::{Serialize, Serializer};

pub mod ser;
pub mod de;
#[cfg(feature = "id_trait")]
pub mod id;
#[cfg(feature = "permissive")]
pub mod permissive;

/// Serialize `trait_object` of type `O` with `serializer`, using `id` as the unique identifier for the concrete type of
/// `trait_object`.
#[inline]
pub fn serialize_trait_object<S, I, O>(
  serializer: S,
  id: I,
  trait_object: &O,
) -> Result<S::Ok, S::Error> where
  S: Serializer,
  I: Serialize,
  O: erased_serde::Serialize + ?Sized,
{
  ser::SerializeTraitObject { id, trait_object }.serialize(serializer)
}

/// Type alias for deserialize functions of trait object type `O`.
pub type DeserializeFn<O> = for<'de> fn(&mut dyn erased_serde::Deserializer<'de>) -> Result<Box<O>, erased_serde::Error>;

/// Registry mapping unique identifiers of types to their deserialize implementations, enabling deserialization of a specific
/// trait object type.
pub trait Registry {
  /// The type of unique identifiers this registry uses. `&'static str` is used as a default.
  type Identifier;
  /// The trait object type this registry maps deserialize functions for.
  type TraitObject: ?Sized;

  /// Register `deserialize_fn` as the deserialize function for `id`.
  ///
  /// If `id` was already registered before, [get_deserialize_fn](Self::get_deserialize_fn) will forever return
  /// `Err(MultipleRegistrations)` for that `id`.
  fn register(&mut self, id: Self::Identifier, deserialize_fn: DeserializeFn<Self::TraitObject>);

  /// Register a default deserialize function for type `T` as the deserialize function for `id`. `T` must implement
  /// [`DeserializeOwned`] and must be convertable into [`Box<Self::TraitObject>`] with
  /// [`Into<Box<Self::TraitObject>>`].
  ///
  /// If `id` was already registered before, [get_deserialize_fn](Self::get_deserialize_fn) will forever return
  /// `Err(MultipleRegistrations)` for that `id`.
  #[inline]
  fn register_type<T>(&mut self, id: Self::Identifier) where
    T: DeserializeOwned + Into<Box<Self::TraitObject>>
  {
    self.register(id, |d| {
      let deserialized = erased_serde::deserialize::<T>(d)?;
      let boxed = deserialized.into();
      Ok(boxed)
    });
  }

  /// Register a default deserialize function for type `T` as the deserialize function for [`T::ID`]. `T` must implement
  /// [`Id`](id::Id) and [`DeserializeOwned`], and must be convertable into [`Box<Self::TraitObject>`] with
  /// [`Into<Box<Self::TraitObject>>`].
  ///
  /// If `T::ID` was already registered before, [get_deserialize_fn](Self::get_deserialize_fn) will forever return
  /// `Err(MultipleRegistrations)` for that `T::ID`.
  #[cfg(feature = "id_trait")]
  #[inline]
  fn register_id_type<T>(&mut self) where
    T: id::Id<Self::Identifier> + DeserializeOwned + Into<Box<Self::TraitObject>>
  {
    self.register_type::<T>(T::ID);
  }

  /// Deserialize a trait object with `deserializer`, using this registry to get the deserialize function for the
  /// concrete type, based on the deserialized ID.
  ///
  /// # Errors
  ///
  /// Returns an error if 0 or more than 1 deserialize functions were registered for the deserialized ID.
  #[inline]
  fn deserialize_trait_object<'de, D>(&self, deserializer: D) -> Result<Box<Self::TraitObject>, D::Error> where
    D: Deserializer<'de>,
    Self: Sized,
    Self::Identifier: Deserialize<'de> + Debug,
  {
    de::DeserializeTraitObject(self).deserialize(deserializer)
  }

  /// Gets the deserialize function for `id`.
  ///
  /// # Errors
  ///
  /// - `GetError::NotRegistered { id }` if no deserialize function was registered for `id`.
  /// - `GetError::MultipleRegistrations { id }` if multiple deserialize functions were registered for `id`.
  fn get_deserialize_fn(&self, id: Self::Identifier) -> Result<&DeserializeFn<Self::TraitObject>, GetError<Self::Identifier>>;

  /// Gets the trait object name, for diagnostic purposes.
  fn get_trait_object_name(&self) -> &'static str;
}

/// Error while getting deserialize function.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum GetError<I> {
  /// No deserialize function was registered for `id`.
  NotRegistered { id: I },
  /// Multiple deserialize functions were registered for `id`.
  MultipleRegistrations { id: I },
}
impl<I: Debug> Error for GetError<I> {}
impl<I: Debug> Display for GetError<I> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      GetError::NotRegistered { id } => write!(f, "no deserialize function was registered for id '{:?}'", id),
      GetError::MultipleRegistrations { id } => write!(f, "multiple deserialize functions were registered for id '{:?}'", id),
    }
  }
}


/// [Registry] implementation mapping unique identifiers of type [`I`] to deserialize functions of trait object type [`O`],
/// using a [BTreeMap].
pub struct MapRegistry<O: ?Sized, I = &'static str> {
  deserialize_fns: BTreeMap<I, Option<DeserializeFn<O>>>,
  trait_object_name: &'static str,
}

impl<O: ?Sized, I> MapRegistry<O, I> {
  /// Creates a new registry, using `trait_object_name` as the name of [`O`] for diagnostic purposes.
  #[inline]
  pub fn new(trait_object_name: &'static str) -> Self {
    Self {
      deserialize_fns: BTreeMap::new(),
      trait_object_name,
    }
  }
}

impl<O: ?Sized, I: Ord> Registry for MapRegistry<O, I> {
  type Identifier = I;
  type TraitObject = O;

  #[inline]
  fn register(&mut self, id: I, deserialize_fn: DeserializeFn<O>) {
    self.deserialize_fns.entry(id)
      .and_modify(|v| { v.take(); })
      .or_insert_with(|| Some(deserialize_fn));
  }

  #[inline]
  fn get_deserialize_fn(&self, id: I) -> Result<&DeserializeFn<O>, GetError<I>> {
    match self.deserialize_fns.get(&id) {
      None => Err(GetError::NotRegistered { id }),
      Some(None) => Err(GetError::MultipleRegistrations { id }),
      Some(Some(deserialize_fn)) => Ok(deserialize_fn),
    }
  }

  #[inline]
  fn get_trait_object_name(&self) -> &'static str {
    self.trait_object_name
  }
}

