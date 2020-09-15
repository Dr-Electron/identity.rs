use crate::Diff;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter, Result as FmtResult};

/// The Diff Type for `Vec`.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiffVec<T: Diff>(pub Vec<InnerVec<T>>);

/// The Inner value for the `DiffVec` type.  Is `untagged` by default for `serde`.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InnerVec<T: Diff> {
    /// logs a change in a `Vec` type.
    Change { index: usize, item: <T as Diff>::Type },
    /// Logs a remove event in a `Vec` type.
    Remove { count: usize },
    /// logs an Add event in a `Vec` type.
    Add(<T as Diff>::Type),
}

impl<T: Diff> DiffVec<T> {
    /// returns an iterator of `&InnerVec<T>` from a `DiffVec<T>`
    pub fn iter<'d>(&'d self) -> Box<dyn Iterator<Item = &InnerVec<T>> + 'd> {
        Box::new(self.0.iter())
    }

    /// returns an iterator of `InnerVec<T>` from a `DiffVec<T>`
    pub fn into_iter<'d>(self) -> Box<dyn Iterator<Item = InnerVec<T>> + 'd>
    where
        Self: 'd,
    {
        Box::new(self.0.into_iter())
    }

    /// Returns the length of the `DiffVec<T>` type.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// `Diff` trait implementation for `Vec<T>`
impl<T> Diff for Vec<T>
where
    T: Clone + Debug + PartialEq + Diff + Default + for<'de> Deserialize<'de> + Serialize,
{
    /// Corresponding Diff Type for `Vec<T>`
    type Type = DiffVec<T>;

    /// Compares two `Vec<T>` types; `self`, `other` and returns a `DiffVec<T>` type.
    fn diff(&self, other: &Self) -> Self::Type {
        let (l_len, r_len) = (self.len(), other.len());
        let max = usize::max(l_len, r_len);
        let mut changes: Vec<InnerVec<T>> = vec![];

        for index in 0..max {
            match (self.get(index), other.get(index)) {
                (None, None) => panic!("No data to match"),
                (Some(x), Some(y)) if x == y => {}
                (Some(x), Some(y)) => changes.push(InnerVec::Change { index, item: x.diff(y) }),
                (None, Some(x)) => changes.push(InnerVec::Add(x.clone().into_diff())),
                (Some(_), None) => match changes.last_mut() {
                    Some(InnerVec::Remove { ref mut count }) => *count += 1,
                    _ => changes.push(InnerVec::Remove { count: 1 }),
                },
            }
        }

        DiffVec(changes)
    }

    /// Merges a `DiffVec<T>`; `diff` with `self`; a `Vec<T>` to create a new `Vec<T>`.
    fn merge(&self, diff: Self::Type) -> Self {
        let mut vec: Self = self.clone();

        for change in diff.into_iter() {
            match change {
                InnerVec::Add(d) => vec.push(<T>::from_diff(d)),
                InnerVec::Change { index, item } => vec[index] = self[index].merge(item),
                InnerVec::Remove { count } => {
                    for _ in 0..count {
                        vec.pop().expect("Unable to pop from the vector");
                    }
                }
            }
        }

        vec
    }

    /// Converts a `DiffVec<T>`; `diff` into a `Vec<T>`.
    fn from_diff(diff: Self::Type) -> Self {
        let mut vec: Vec<T> = vec![];

        for (index, elm) in diff.0.into_iter().enumerate() {
            match elm {
                InnerVec::Add(add) => vec.push(<T>::from_diff(add)),
                _ => panic!("Invalid Diff {:?}", index),
            }
        }

        vec
    }

    /// Converts a `Vec<T>` into a `DiffVec<T>`
    fn into_diff(self) -> Self::Type {
        let mut changes: Vec<InnerVec<T>> = vec![];
        for inner in self {
            changes.push(InnerVec::Add(inner.into_diff()));
        }
        DiffVec(changes)
    }
}

/// Debug trait for `DiffVec<T>`
impl<T: Diff> Debug for DiffVec<T> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "DiffVec ")?;
        f.debug_list().entries(self.0.iter()).finish()
    }
}

/// Debug trait for `InnerVec<T>`
impl<T: Diff> Debug for InnerVec<T> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match &self {
            Self::Change { index, item } => f
                .debug_struct("Change")
                .field("index", index)
                .field("item", item)
                .finish(),
            Self::Remove { count } => f.debug_struct("Remove").field("count", count).finish(),
            Self::Add(diff) => write!(f, "Add({:#?})", diff),
        }
    }
}

/// Default trait for `DiffVec<T>`
impl<T: Diff> Default for DiffVec<T> {
    fn default() -> Self {
        DiffVec(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_same_val() {
        let vec_a: Vec<i32> = vec![1, 2, 3];
        let vec_b: Vec<i32> = vec![1, 2, 3];

        assert_eq!(vec_a, vec_b);

        let diff = vec_a.diff(&vec_b);

        assert_eq!(diff, DiffVec(vec![]));

        let vec_c = vec_a.merge(diff);

        assert_eq!(vec_b, vec_c);

        let diff = vec_b.diff(&vec_a);

        assert_eq!(diff, DiffVec(vec![]));

        let vec_c = vec_b.merge(diff);

        assert_eq!(vec_a, vec_c);
    }

    #[test]
    fn test_different_vals() {
        let vec_a = vec![1, 2, 3, 4, 5];
        let vec_b = vec![4, 2, 3, 4, 6];

        let diff = vec_a.diff(&vec_b);

        assert_eq!(
            diff,
            DiffVec(vec![
                InnerVec::Change {
                    index: 0,
                    item: 4i32.into_diff(),
                },
                InnerVec::Change {
                    index: 4,
                    item: 6i32.into_diff(),
                }
            ])
        );

        let vec_c = vec_a.merge(diff);

        assert_eq!(vec_b, vec_c);

        let diff = vec_b.diff(&vec_a);

        assert_eq!(
            diff,
            DiffVec(vec![
                InnerVec::Change {
                    index: 0,
                    item: 1i32.into_diff(),
                },
                InnerVec::Change {
                    index: 4,
                    item: 5i32.into_diff(),
                }
            ])
        );

        let vec_c = vec_b.merge(diff);

        assert_eq!(vec_a, vec_c);
    }

    #[test]
    fn test_diff_lengths() {
        let vec_a = vec![1, 2, 3, 4, 5, 6];
        let vec_b = vec![1, 2, 3, 4, 6, 7, 8];

        let diff = vec_a.diff(&vec_b);

        assert_eq!(
            diff,
            DiffVec(vec![
                InnerVec::Change {
                    index: 4,
                    item: 6i32.into_diff(),
                },
                InnerVec::Change {
                    index: 5,
                    item: 7i32.into_diff(),
                },
                InnerVec::Add(8.into_diff())
            ])
        );

        let vec_c = vec_a.merge(diff);

        assert_eq!(vec_b, vec_c);

        let diff = vec_b.diff(&vec_a);

        assert_eq!(
            diff,
            DiffVec(vec![
                InnerVec::Change {
                    index: 4,
                    item: 5i32.into_diff(),
                },
                InnerVec::Change {
                    index: 5,
                    item: 6i32.into_diff(),
                },
                InnerVec::Remove { count: 1 },
            ])
        );

        let vec_c = vec_b.merge(diff);

        assert_eq!(vec_a, vec_c);
    }
}
