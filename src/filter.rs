#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum Filter<T> {
    #[default]
    Any,
    Is(T),
}
