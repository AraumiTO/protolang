use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash, Default)]
pub struct Span {
  pub start: usize,
  pub end: usize,
  pub line: usize,
  pub column: usize,
}

impl Span {
  pub fn identity() -> Self {
    Self {
      start: 0,
      end: 0,
      line: 0,
      column: 0
    }
  }

  pub fn between(&self, to: Self) -> Self {
    Span {
      start: self.start,
      end: to.end,
      line: self.line,
      column: self.column,
    }
  }

  pub fn wrap<T>(self, value: T) -> Positioned<T> {
    Positioned { value, span: self }
  }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Positioned<T> {
  pub value: T,
  pub span: Span,
}

impl<T> Positioned<T> {
  pub fn new(value: T, span: Span) -> Positioned<T> {
    Positioned { value, span }
  }

  pub fn identity(value: T) -> Positioned<T> {
    Positioned { value, span: Span::identity() }
  }

  pub fn between<U>(&self, value: &Positioned<U>) -> Span {
    self.span.between(value.span)
  }

  pub fn wrap<U>(&self, value: U) -> Positioned<U> {
    self.span.wrap(value)
  }

  pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Positioned<U> {
    self.span.wrap(f(self.value))
  }

  pub fn unpack(self) -> (Span, T) {
    (self.span, self.value)
  }
}

impl<T: Debug> Debug for Positioned<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.value)
  }
}
