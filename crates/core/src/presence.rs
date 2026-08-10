//! ValueState<T> = Present(T) | Absent | NotModelled   (pure)

/// Describes whether a requested value exists in the producing model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueState<T> {
    /// The value exists and its payload is authoritative.
    Present(T),
    /// No value is present at the requested position.
    Absent,
    /// The requested concept was outside the model that produced the data.
    NotModelled,
}
