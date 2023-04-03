#![deny(clippy::use_self)]

use std::{marker::PhantomData, num::NonZeroU32};

#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
mod size_asserts {
    use crate::{static_assert_size, List};

    static_assert_size!(List<String>, 4);
    static_assert_size!(Option<List<String>>, 4);
}

pub struct List<T> {
    node: ListNodePtr<T>,
}

pub struct Node<T> {
    head: T,
    tail: ListNodePtr<T>,
}

impl<T> List<T> {
    pub fn peek<'a>(&self, arena: &'a Arena<T>) -> Option<&'a T> {
        (self.node != ListNodePtr::INVALID).then(|| &arena.data(self.node).head)
    }

    pub fn is_empty(&self) -> bool {
        self.node == ListNodePtr::INVALID
    }

    pub fn push_front(&mut self, arena: &mut Arena<T>, head: T) {
        self.node = arena.add(Node {
            head,
            tail: self.node,
        });
    }

    pub fn pop_front<'a>(&mut self, arena: &'a Arena<T>) -> Option<&'a T> {
        if self.is_empty() {
            return None;
        }

        let node = arena.data(self.node);
        self.node = node.tail;
        Some(&node.head)
    }

    pub fn iter(mut self, arena: &Arena<T>) -> impl Iterator<Item = &T> {
        std::iter::from_fn(move || self.pop_front(arena))
    }
}

impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        Self { node: self.node }
    }
}
impl<T> Copy for List<T> {}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self {
            node: ListNodePtr::INVALID,
        }
    }
}

pub struct Arena<T> {
    values: Vec<Node<T>>,
}

impl<T> Arena<T> {
    fn add(&mut self, t: Node<T>) -> ListNodePtr<T> {
        self.values.push(t);

        let len = self.values.len();
        assert!(len < ListNodePtr::<T>::MAX_USIZE);
        unsafe { ListNodePtr::new_unchecked(len as u32) }
    }

    fn data(&self, ptr: ListNodePtr<T>) -> &Node<T> {
        &self.values[ptr.as_usize()]
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self { values: Vec::new() }
    }
}

struct ListNodePtr<T> {
    index: NonZeroU32,
    marker: PhantomData<T>,
}

impl<T> Clone for ListNodePtr<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            marker: self.marker,
        }
    }
}
impl<T> Copy for ListNodePtr<T> {}

impl<T> PartialEq for ListNodePtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for ListNodePtr<T> {}

impl<T> ListNodePtr<T> {
    const INVALID: Self = unsafe { Self::new_unchecked(Self::MAX_U32) };
    const MAX_U32: u32 = std::u32::MAX - 0xFF;
    const MAX_USIZE: usize = Self::MAX_U32 as usize;

    const unsafe fn new_unchecked(index: u32) -> Self {
        Self {
            index: NonZeroU32::new_unchecked(index),
            marker: PhantomData,
        }
    }

    fn as_usize(self) -> usize {
        (self.index.get() - 1) as usize
    }
}

#[macro_export]
macro_rules! static_assert_size {
    ($ty:ty, $size:expr) => {
        const _: [(); $size] = [(); ::std::mem::size_of::<$ty>()];
    };
}

#[cfg(test)]
mod test {
    use super::{Arena, List};

    #[test]
    fn basics() {
        let mut arena = Arena::default();
        let mut list = List::default();

        // Check empty list behaves right
        assert!(list.is_empty());
        assert_eq!(list.peek(&arena), None);
        assert_eq!(list.pop_front(&arena), None);

        // Populate list
        list.push_front(&mut arena, 1);
        assert_eq!(list.peek(&arena), Some(&1));

        list.push_front(&mut arena, 2);
        assert_eq!(list.peek(&arena), Some(&2));

        list.push_front(&mut arena, 3);
        assert_eq!(list.peek(&arena), Some(&3));

        assert_eq!(list.iter(&arena).copied().collect::<Vec<_>>(), &[3, 2, 1]);

        // Check normal removal
        assert_eq!(list.pop_front(&arena), Some(&3));
        assert_eq!(list.pop_front(&arena), Some(&2));

        // Push some more just to make sure nothing's corrupted
        list.push_front(&mut arena, 4);
        list.push_front(&mut arena, 5);

        // Check normal removal
        assert_eq!(list.pop_front(&arena), Some(&5));
        assert_eq!(list.pop_front(&arena), Some(&4));

        // Check exhaustion
        assert_eq!(list.pop_front(&arena), Some(&1));
        assert_eq!(list.pop_front(&arena), None);
    }
}
