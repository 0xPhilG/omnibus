/// A subscription filter for one dimension (origin or kind) of an [`crate::Event`].
///
/// Pass two `Filter` values to [`crate::Bus::subscribe`] to control which events a
/// subscription receives — one for the origin dimension and one for the kind
/// dimension.
///
/// # Examples
///
/// ```rust
/// use omnibus::Filter;
///
/// // Match any value for this dimension:
/// let any: Filter<String> = Filter::Any;
///
/// // Match only "sensor":
/// let exact = Filter::Is("sensor".to_string());
///
/// assert_ne!(any, exact);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum Filter<T> {
    /// Matches any value; events pass this filter regardless of their value.
    #[default]
    Any,
    /// Matches only events whose origin or kind equals the wrapped value.
    Is(T),
}
