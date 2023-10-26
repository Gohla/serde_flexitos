/// Get a unique and stable identifier for a type, used for (de)serialization of trait objects.
pub trait Id {
  /// The unique and stable identifier of this type.
  const ID: &'static str;
}

/// Get a unique and stable identifier for the concrete type of a value, used for (de)serialization of trait objects.
///
/// Object safe proxy of [`Id`].
pub trait IdObj {
  /// Gets the unique and stable identifier for the concrete type of this value.
  fn id(&self) -> &'static str;
}

// Implement IdObj for all types that implement Id.
impl<T: Id + ?Sized> IdObj for T {
  #[inline]
  fn id(&self) -> &'static str { T::ID }
}

// Implement Id for standard library types

impl Id for () {
  const ID: &'static str = "()";
}
impl Id for bool {
  const ID: &'static str = "bool";
}
impl Id for str {
  const ID: &'static str = "str";
}
impl Id for String {
  const ID: &'static str = "String";
}
