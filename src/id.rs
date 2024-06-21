use std::fmt::{Display, Formatter, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An identifier consisting of one or two string slices.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum Ident<'a> { // TODO: support more than 3 elements.
  I1(&'a str),
  I2(&'a str, &'a str),
  I3(&'a str, &'a str, &'a str),
}

/// Get a unique and stable identifier (of type `I`) for a type, used for (de)serialization of trait objects.
pub trait Id<I = Ident<'static>> {
  /// The unique and stable identifier of this type.
  const ID: I;
}

/// Get a unique and stable identifier (of type `I`) for the concrete type of a value, used for (de)serialization of
/// trait objects.
///
/// Object safe proxy of [`Id`].
pub trait IdObj<I = Ident<'static>> {
  /// Gets the unique and stable identifier for the concrete type of this value.
  fn id(&self) -> I;
}


/// Create an `Ident` from expressions.
#[macro_export]
macro_rules! ident {
  ($a:expr) => { $crate::id::Ident::I1($a) };
  ($a:expr, $b:expr) => { $crate::id::Ident::I2($a, $b) };
  ($a:expr, $b:expr, $c:expr) => { $crate::id::Ident::I3($a, $b, $c) };
}

/// Create an `Ident` from a concrete type or an instantiated generic type with one or two type argument.
#[macro_export]
macro_rules! type_to_ident {
  ($generic:ident<$arg_a:ty, $arg_b:ty>) => {
    $crate::ident!(stringify!($generic), stringify!($arg_a), stringify!($arg_b))
  };
  ($generic:ident<$arg_a:ty>) => {
    $crate::ident!(stringify!($generic), stringify!($arg_a))
  };
  ($concrete:ty) => {
    $crate::ident!(stringify!($concrete))
  };
}

impl<'a> Ident<'a> {
  /// Append `other` to this ident if there is space. Panics if there is no more space.
  pub const fn append(self, other: &'a str) -> Ident<'a> {
    match self {
      Ident::I1(a) => Ident::I2(a, other),
      Ident::I2(a, b) => Ident::I3(a, b, other),
      _ => panic!("can't append; `Ident` can only have at most 3 elements"), // Can't include idents in panic messages, as const formatting has not been stabilized.
    }
  }

  /// Extend this ident with `other` if there is space in this ident. Panics if there is no more space.
  pub const fn extend(self, other: Ident<'a>) -> Ident<'a> {
    match (self, other) {
      (Ident::I1(a), Ident::I1(b)) => Ident::I2(a, b),
      (Ident::I2(a, b), Ident::I1(c)) => Ident::I3(a, b, c),
      (Ident::I1(a), Ident::I2(b, c)) => Ident::I3(a, b, c),
      _ => panic!("can't extend; `Ident` can only have at most 3 elements"), // Can't include idents in panic messages, as const formatting has not been stabilized.
    }
  }
}


// Manually serialize and deserialize as strings, enabling usage as JSON map keys.
const SEPARATOR: char = '/';
impl Display for Ident<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Ident::I1(a) => f.write_str(a),
      Ident::I2(a, b) => {
        f.write_str(a)?;
        f.write_char(SEPARATOR)?;
        f.write_str(b)
      }
      Ident::I3(a, b, c) => {
        f.write_str(a)?;
        f.write_char(SEPARATOR)?;
        f.write_str(b)?;
        f.write_char(SEPARATOR)?;
        f.write_str(c)
      }
    }
  }
}
impl Serialize for Ident<'_> {
  #[inline]
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(self)
  }
}
impl<'de> Deserialize<'de> for Ident<'de> { // Returned ident borrows from deserializer
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let str = <&str>::deserialize(deserializer)?;
    let ident = if let Some(idx_a) = str.find(SEPARATOR) {
      let (a, b) = str.split_at(idx_a);
      let b_no_sep = &b[1..];
      if let Some(idx_b) = b_no_sep.find(SEPARATOR) {
        let (b, c) = str.split_at(idx_b);
        let c_no_sep = &c[1..];
        ident!(a, b, c_no_sep)
      } else {
        ident!(a, b_no_sep)
      }
    } else {
      ident!(str)
    };
    Ok(ident)
  }
}


// Implement `IdObj` for all types that implement `Id`.
impl<I, D: Id<I> + ?Sized> IdObj<I> for D {
  #[inline]
  fn id(&self) -> I { D::ID }
}

// Implement `Id` for standard library types

// TODO: exhaustively implement `Id` for all standard library types

macro_rules! impl_id {
  ($ty:ty) => {
    impl $crate::id::Id<&'static str> for $ty {
      const ID: &'static str = stringify!($ty);
    }
    impl $crate::id::Id<$crate::id::Ident<'static>> for $ty {
      const ID: $crate::id::Ident<'static> = $crate::ident!(<Self as Id<&'static str>>::ID);
    }
  };
}

impl_id!(());
impl_id!(bool);
impl_id!(char);
impl_id!(u8);
impl_id!(u16);
impl_id!(u32);
impl_id!(u64);
impl_id!(u128);
impl_id!(usize);
impl_id!(i8);
impl_id!(i16);
impl_id!(i32);
impl_id!(i64);
impl_id!(i128);
impl_id!(isize);
impl_id!(f32);
impl_id!(f64);
impl_id!(str);

impl_id!(String);
impl_id!(PathBuf);
impl_id!(Path);
impl_id!(SystemTime);

impl<T: Id> Id for [T] {
  const ID: Ident<'static> = Ident::I1("[]").extend(T::ID);
}
impl<T: Id, const N: usize> Id for [T; N] {
  const ID: Ident<'static> = Ident::I1("[]").append(stringify!(N)).extend(T::ID);
}

impl<T: Id> Id for &T {
  const ID: Ident<'static> = Ident::I1("&").extend(T::ID);
}
impl<T: Id> Id for &mut T {
  const ID: Ident<'static> = Ident::I1("&mut").extend(T::ID);
}
impl<T: Id> Id for &[T] {
  const ID: Ident<'static> = Ident::I1("&[]").extend(T::ID);
}
impl<T: Id> Id for &mut [T] {
  const ID: Ident<'static> = Ident::I1("&mut []").extend(T::ID);
}

impl<T: Id> Id for Option<T> {
  const ID: Ident<'static> = Ident::I1("Option").extend(T::ID);
}
impl<T: Id, E: Id> Id for Result<T, E> {
  const ID: Ident<'static> = Ident::I1("Result").extend(T::ID).extend(E::ID);
}

impl<T: Id> Id for Box<T> {
  const ID: Ident<'static> = Ident::I1("Box").extend(T::ID);
}
impl<T: Id> Id for Rc<T> {
  const ID: Ident<'static> = Ident::I1("Rc").extend(T::ID);
}
impl<T: Id> Id for Arc<T> {
  const ID: Ident<'static> = Ident::I1("Arc").extend(T::ID);
}

impl<T: Id> Id for Vec<T> {
  const ID: Ident<'static> = Ident::I1("Vec").extend(T::ID);
}
