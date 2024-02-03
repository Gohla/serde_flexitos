/// Get a unique and stable identifier (of type `I`) for a type, used for (de)serialization of trait objects.
pub trait Id<I = &'static str> {
  /// The unique and stable identifier of this type.
  const ID: I;
}

/// Get a unique and stable identifier (of type `I`) for the concrete type of a value, used for (de)serialization of
/// trait objects.
///
/// Object safe proxy of [`Id`].
pub trait IdObj<I = &'static str> {
  /// Gets the unique and stable identifier for the concrete type of this value.
  fn id(&self) -> I;
}

// Implement IdObj for all types that implement Id.
impl<I, D: Id<I> + ?Sized> IdObj<I> for D {
  #[inline]
  fn id(&self) -> I { D::ID }
}

// Implement Id for standard library types

impl Id for () {
  const ID: &'static str = "()";
}
impl Id for bool {
  const ID: &'static str = "bool";
}
impl Id for usize {
  const ID: &'static str = "usize";
}
impl Id for str {
  const ID: &'static str = "str";
}
impl Id for String {
  const ID: &'static str = "String";
}
