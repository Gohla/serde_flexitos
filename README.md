# Serde FlexiTOS

[![crates.io](https://img.shields.io/crates/v/serde_flexitos)](https://crates.io/crates/serde_flexitos)
[![docs](https://docs.rs/serde_flexitos/badge.svg)](https://docs.rs/serde_flexitos)
[![GitHub License](https://img.shields.io/github/license/Gohla/serde_flexitos)](https://github.com/Gohla/serde_flexitos/blob/main/LICENSE)

<!-- cargo-rdme start -->

This crate provides types and function for flexible serialization and deserialization of trait objects with
[serde][serde].

## Why?

When you need to treat several types that implement a trait *as a single type*, a trait object type `dyn O` is one
of the most convenient solutions in Rust. If you need (de)serialization for such a trait object, this crate can help
provide that.

## Is this not already possible?

Serializing a trait object is already possible with [erased-serde][erased-serde]'s [`erased_serde::Serialize`].
However, erased-serde does not provide a convenient way to *deserialize trait objects*.

To deserialize a trait object, we first need to figure out the concrete type that was serialized, and then use the
corresponding [`Deserialize`] implementation of that type to deserialize the value. We cannot use the trait object
type directly to get the corresponding [`Deserialize`] implementation, because trait objects must be object safe,
ruling out associated functions (only allowing methods: functions that take a `&self` and variations). But when
deserializing, we do not have an instance of the trait object (we are instantiating it with deserialization!), thus
there is no method to call. Therefore, an external mechanism is needed to get [`Deserialize`] implementations for
concrete types.

Two other solutions exist (to my knowledge), but they make trade-offs that not everyone is willing to make, and
provide no way to opt-out of those trade-offs:
- [typetag][typetag]: A convenient solution to get (de)serialization for trait objects. However, it uses
  [inventory][inventory] to register [`Deserialize`] implementations, which does not work on every platform (for
  example, WASM). It also registers these implementations globally using a procedural macro that has to be applied
  to every concrete type. Finally, generic traits and generic impls of traits are not supported.
  If you can work within these limitations, [typetag][typetag] is a great crate, and you should probably use it
  instead of this one because it is more convenient!
- [serde_traitobject][serde_traitobject]: An interesting solution that (de)serializes the entire vtable of a trait
  object. However, the vtable is not stable across different compilations, changes when you add or remove
  implementations of your trait object, and requires nightly Rust. Therefore, this crate is only usable in the very
  limited scope of sending trait objects to another process of the same binary, but is very convenient for that
  specific use-case.

This crate provides types and functions for flexible (de)serialization of trait objects, that do not necessarily
require macros, global registration, nightly rust, nor require your binary not to change, at the cost of some
convenience. However, convenience can be brought back by creating layers on top of this crate, such as a global
registration macro, allowing you to make the trade-off between convenience and flexibility yourself.

## How does it work?

A trait object is serialized as an id-value pair, also known as the [externally tagged enum representation][exttag],
where the id is the *unique identifier* for the concrete type of the value, and the value is serialized using the
trait object's [`erased_serde::Serialize`] implementation.

A trait object is deserialized by first deserializing the ID, then finding the [`Deserialize`] implementation of
the concrete type using that ID, and then deserializing the value with that deserialize impl.

An ID must uniquely identify a concrete type of a trait object, and be stable over time, in order for
deserialization to keep working over time. Missing IDs will result in recoverable errors during deserialization.
Duplicate IDs by default also result in recoverable errors during deserialization, but this behaviour can be
customized; see [Error Handling](#error-handling).

## How do I use this crate?

A [`Registry`] handles registration of [`Deserialize`] impls and finding them by ID. For each trait object
you wish to deserialize, you must construct a registry and register all concrete types with it. [`MapRegistry`] is
the standard registry implementation that maps IDs to deserialize impls.

To register a concrete type, we must provide:
1) the ID (`&'static str`) for that concrete type,
2) a deserialize function that deserializes the concrete type as a boxed trait object.

Traits must have [`erased_serde::Serialize`] as a supertrait and have a method to retrieve the ID of the concrete
type. Concrete types of the trait must implement [`Serialize`].

Then, you can implement [`Serialize`] for `dyn Trait` using  [`serialize_trait_object`], and [`Deserialize`] for
`Box<dyn Trait>` using `deserialize_trait_object`.

## Example

An example, using a global registry to get some convenience:

```rust
use std::sync::LazyLock;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use serde_flexitos::{MapRegistry, Registry, serialize_trait_object};

// Trait we want to serialize trait objects of. This example just uses `Debug` as supertrait so we can
// print values.

pub trait Example: erased_serde::Serialize + std::fmt::Debug {
  // Gets the ID uniquely identifying the concrete type of this value. Must be a method for object
  // safety.
  fn id(&self) -> &'static str;
}

// Implementations of the `Example` trait.

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Foo(String);
impl Foo {
  const ID: &'static str = "Foo";
}
impl Example for Foo {
  fn id(&self) -> &'static str { Self::ID }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Bar(usize);
impl Bar {
  const ID: &'static str = "Bar";
}
impl Example for Bar {
  fn id(&self) -> &'static str { Self::ID }
}

// Create registry for `Example` and register all concrete types with it. Store in static with
// `LazyLock` to lazily initialize it once while being able to create global references to it.

static EXAMPLE_REGISTRY: LazyLock<MapRegistry<dyn Example>> = LazyLock::new(|| {
  let mut registry = MapRegistry::<dyn Example>::new("Example");
  registry.register(Foo::ID, |d| Ok(Box::new(erased_serde::deserialize::<Foo>(d)?)));
  registry.register(Bar::ID, |d| Ok(Box::new(erased_serde::deserialize::<Bar>(d)?)));
  registry
});

// (De)serialize implementations

impl<'a> Serialize for dyn Example + 'a {
  fn serialize<S: Serializer >(&self, serializer: S) -> Result<S::Ok, S::Error> {
    // Check that `Example` has `erased_serde::Serialize` as a supertrait, preventing infinite
    // recursion at runtime.
    const fn __check_erased_serialize_supertrait<T: ?Sized + Example>() {
      serde_flexitos::ser::require_erased_serialize_impl::<T>();
    }
    serialize_trait_object(serializer, self.id(), self)
  }
}

impl<'de> Deserialize<'de> for Box<dyn Example> {
  fn deserialize<D: Deserializer<'de> >(deserializer: D) -> Result<Self, D::Error> {
    EXAMPLE_REGISTRY.deserialize_trait_object(deserializer)
  }
}

// Run serialization roundtrip

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let examples: Vec<Box<dyn Example>> = vec![Box::new(Foo("A".to_string())), Box::new(Bar(0))];
  println!("Examples: {:?}", examples);
  let json = serde_json::to_string(&examples)?;
  println!("Serialized: {}", json);
  let roundtrip: Vec<Box<dyn Example>> = serde_json::from_str(&json)?;
  println!("Deserialized: {:?}", roundtrip);
  Ok(())
}
```

See `examples/simple.rs` for a full version of the above example.

## Error Handling

Registration is infallible because registration can be done in static initializers, and dealing with errors in
static initialization functions is awkward. Instead, registration failures are propagated to deserialization-time,
depending on which [`Registry`] implementation is used.

Deserialization of trait objects is fallible because deserializing any concrete type is fallible, for example if the
serialized data is malformed. Additionally, deserialization can fail when:

1) The serialized data contains an ID for which no deserialize impl was registered. This occurs when
   [`Registry::get_deserialize_fn`] returns [`GetError::NotRegistered`]. This is an error because we cannot
   deserialize anything without a corresponding deserialize impl.
2) The serialized data contains an ID for which multiple deserialize impls were registered. This occurs when
   [`Registry::get_deserialize_fn`] returns [`GetError::MultipleRegistrations`]. This is an error because we don't
   know which of the deserialize impls we need to use.

Whether [`Registry::get_deserialize_fn`] returns one of these errors depends on the implementation. The standard
[`MapRegistry`] implementation returns these errors as a safe default. You can create your own [`Registry`]
implementation if you want different behaviour. For example, a registry that ignores multiple registrations and
instead chooses the first registration. See `examples/first_registration.rs` for an example of that.

Finally, serialization of trait objects is fallible because serializing the concrete type behind the trait object
is fallible. Additionally, serialization could fail due to the serializer not being able to serialize an ID. For
example, JSON only supports maps (key-value pairs) with string keys, and would thus fail with IDs that cannot be
serialized to a string.

## Examples

Check out the examples in the `examples` directory for more use-cases:

- `examples/simple.rs`: A full version of the above example.
- `examples/combined.rs`: Define 2 traits, then combine both traits as boxed trait objects in a struct, and
  (de)serialize that struct. This shows how trait objects can be combined/composed.
- `examples/first_registration.rs`: Custom [`Registry`] implementation that ignores multiple registrations and
  instead chooses the first registration
- `examples/macros.rs`: Convenience macro layered on top of this crate, using [linkme][linkme] to register types.
- `examples/no_global.rs`: Use a local registry instead of a global one, using [`DeserializeSeed`] implementations
  provided by this crate.
- `examples/generic_instantiations.rs`: Create and use registries for _instantiations_ of generic traits/structs.
  Does not handle traits nor structs generically though!

## Experimental Features

This library has experimental features that are unstable and work-in-progress. Enable and use these features at your
own risk.

- `permissive`: [`DeserializeSeed`] and [`Visitor`] implementations for permissive deserialization.
- `id`: Trait, macros, and implementations for unique and stable type identifiers.

## Limitations

### Generic Serialization and Deserialization

Serialization and deserialization of trait objects with generic type parameters and structs with generic type
parameters is not supported, and I think it is impossible to support this. To support this, we would need a function
that goes from a run-time unique identifier (`&str`) to a compile-time type, but that is impossible. This makes
sense, because the compiler needs to know at compile-time which (combination of) concrete types are used as generic
type arguments to do monomorphization.

However, it is possible to register all concrete instances of types that you wish to deserialize, as is done in
`example/generic_instantiations.rs`.

### Other Representations

Only the [externally tagged enum representation][exttag] is supported for (de)serializing trait objects, to simplify
the implementations in this crate. This is only a problem if you need to accept serialized trait objects that were
serialized externally using a different representation (i.e., not this crate).

## Inspiration

This crate is inspired by the excellent [typetag][typetag] crate.

[serde]: https://crates.io/crates/serde
[erased-serde]: https://crates.io/crates/erased-serde
[exttag]: https://serde.rs/enum-representations.html#externally-tagged
[typetag]: https://crates.io/crates/typetag
[linkme]: https://crates.io/crates/linkme
[inventory]: https://crates.io/crates/inventory
[objs]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
[serde_traitobject]: https://crates.io/crates/serde_traitobject

<!-- cargo-rdme end -->

[`serialize_trait_object`]: https://docs.rs/serde_flexitos/latest/serde_flexitos/fn.serialize_trait_object.html
[`Registry`]: https://docs.rs/serde_flexitos/latest/serde_flexitos/trait.Registry.html
[`Registry::get_deserialize_fn`]: https://docs.rs/serde_flexitos/latest/serde_flexitos/trait.Registry.html#tymethod.get_deserialize_fn
[`GetError::NotRegistered`]: https://docs.rs/serde_flexitos/latest/serde_flexitos/enum.GetError.html#variant.NotRegistered
[`GetError::MultipleRegistrations`]: https://docs.rs/serde_flexitos/latest/serde_flexitos/enum.GetError.html#variant.MultipleRegistrations
[`MapRegistry`]: https://docs.rs/serde_flexitos/latest/serde_flexitos/struct.MapRegistry.html

[`Serialize`]: https://docs.rs/serde/latest/serde/trait.Serialize.html
[`Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[`DeserializeSeed`]: https://docs.rs/serde/latest/serde/de/trait.DeserializeSeed.html

[`erased_serde::Serialize`]: https://docs.rs/erased-serde/latest/erased_serde/trait.Serialize.html
