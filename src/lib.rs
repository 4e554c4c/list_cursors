#![allow(dead_code)]
#![feature(box_into_raw_non_null)]
#![feature(box_syntax)]
use std::fmt;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// A doubly-linked list with owned nodes.
///
/// The `LinkedList` allows pushing and popping elements at either end
/// in constant time.
///
/// This is the same `LinkedList` used in `alloc`
pub struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
    tail: Option<NonNull<Node<T>>>,
    len: usize,
    marker: PhantomData<Box<Node<T>>>,
}

struct Node<T> {
    next: Option<NonNull<Node<T>>>,
    prev: Option<NonNull<Node<T>>>,
    element: T,
}
impl<T> Node<T> {
    fn new(element: T) -> Self {
        Node {
            next: None,
            prev: None,
            element,
        }
    }

    fn into_element(self: Box<Self>) -> T {
        self.element
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        LinkedList {
            head: None,
            tail: None,
            len: 0,
            marker: PhantomData,
        }
    }
    /// Provides a cursor to the empty element
    pub fn cursor(&self) -> Cursor<T> {
        Cursor {
            list: self,
            current: None,
        }
    }

    /// Provides a cursor with mutable references and access to the list
    pub fn cursor_mut(&mut self) -> CursorMut<T> {
        CursorMut {
            list: self,
            current: None,
            current_len: 0,
        }
    }
    /* other list methods go here */
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut c = self.cursor_mut();
        while c.pop().is_some() {}
    }
}

impl<T: fmt::Debug> fmt::Debug for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut t = f.debug_list();
        let mut c = self.cursor();
        c.move_next();
        while let Some(e) = c.current() {
            t.entry(&e);
            c.move_next();
        }

        t.finish()
    }
}

impl<T> FromIterator<T> for LinkedList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> LinkedList<T> {
        let mut list = LinkedList::new();
        {
            let mut cursor = list.cursor_mut();
            for el in iter {
                cursor.insert_before(el);
            }
        }
        list
    }
}

/// An Immutable look into a `LinkedList` that can be moved back and forth
pub struct Cursor<'list, T: 'list> {
    current: Option<NonNull<Node<T>>>,
    list: &'list LinkedList<T>,
}

impl<'list, T> Cursor<'list, T> {
    fn next(&self) -> Option<NonNull<Node<T>>> {
        self.current
            .map_or(self.list.head, |node| unsafe { node.as_ref().next })
    }
    fn prev(&self) -> Option<NonNull<Node<T>>> {
        self.current
            .map_or(self.list.tail, |node| unsafe { node.as_ref().prev })
    }
    /// Move to the subsequent element of the list if it exists or the empty
    /// element
    pub fn move_next(&mut self) {
        self.current = self.next()
    }
    /// Move to the previous element of the list
    pub fn move_prev(&mut self) {
        self.current = self.prev();
    }

    /// Get the current element
    pub fn current(&self) -> Option<&'list T> {
        self.current.map(|node| unsafe {
            // Need an unbound lifetime to get 'list
            let node = &*node.as_ptr();
            &node.element
        })
    }
    /// Get the following element
    pub fn peek(&self) -> Option<&'list T> {
        self.next().map(|next_node| unsafe {
            let next_node = &*next_node.as_ptr();
            &next_node.element
        })
    }
    /// Get the previous element
    pub fn peek_before(&self) -> Option<&'list T> {
        self.prev().map(|prev_node| unsafe {
            let prev_node = &*prev_node.as_ptr();
            &prev_node.element
        })
    }
}

/// A mutable view into a `LinkedList` that can be used to edit the collection
pub struct CursorMut<'list, T: 'list> {
    current: Option<NonNull<Node<T>>>,
    list: &'list mut LinkedList<T>,
    current_len: usize,
}

impl<'list, T> CursorMut<'list, T> {
    fn next(&self) -> Option<NonNull<Node<T>>> {
        self.current
            .map_or(self.list.head, |node| unsafe { node.as_ref().next })
    }
    fn prev(&self) -> Option<NonNull<Node<T>>> {
        self.current
            .map_or(self.list.tail, |node| unsafe { node.as_ref().prev })
    }
    // `current_len` is in the range 0...self.list.len at all times
    fn inc_len(&mut self) {
        self.current_len += 1;
        self.current_len %= self.list.len + 1;
    }
    fn dec_len(&mut self) {
        self.current_len += self.list.len;
        self.current_len %= self.list.len + 1;
    }

    /// Move to the subsequent element of the list if it exists or the empty
    /// element
    pub fn move_next(&mut self) {
        self.inc_len();
        self.current = self.next()
    }
    /// Move to the previous element of the list
    pub fn move_prev(&mut self) {
        self.dec_len();
        self.current = self.prev()
    }

    /// Get the current element
    pub fn current(&mut self) -> Option<&mut T> {
        self.current.map(|node| unsafe {
            // Need an unbound lifetime to get same lifetime as self
            let node = &mut *node.as_ptr();
            &mut node.element
        })
    }
    /// Get the next element
    pub fn peek(&mut self) -> Option<&mut T> {
        self.next().map(|next_node| unsafe {
            let next_node = &mut *next_node.as_ptr();
            &mut next_node.element
        })
    }
    /// Get the previous element
    pub fn peek_before(&self) -> Option<&mut T> {
        self.prev().map(|prev_node| unsafe {
            let prev_node = &mut *prev_node.as_ptr();
            &mut prev_node.element
        })
    }

    /// Get an immutable cursor at the current element
    pub fn as_cursor(&self) -> Cursor<T> {
        Cursor {
            current: self.current,
            list: self.list,
        }
    }

    // Now the list editing operations

    /// Insert `item` after the cursor
    pub fn insert(&mut self, item: T) {
        let mut node = box Node::new(item);
        node.prev = self.current;
        node.next = self.next();

        unsafe {
            let node_ptr = Some(Box::into_raw_non_null(node));
            match self.next() {
                None => self.list.tail = node_ptr,
                Some(mut next) => next.as_mut().prev = node_ptr,
            }
            match self.current {
                None => self.list.head = node_ptr,
                Some(mut prev) => prev.as_mut().next = node_ptr,
            }
        }
        self.list.len += 1;
    }
    /// Insert `item` before the cursor
    pub fn insert_before(&mut self, item: T) {
        let mut node = box Node::new(item);
        node.prev = self.prev();
        node.next = self.current;

        unsafe {
            let node_ptr = Some(Box::into_raw_non_null(node));
            match self.prev() {
                None => self.list.head = node_ptr,
                Some(mut next) => next.as_mut().next = node_ptr,
            }
            match self.current {
                None => self.list.tail = node_ptr,
                Some(mut prev) => prev.as_mut().prev = node_ptr,
            }
        }
        self.list.len += 1;
        self.inc_len();
    }

    /// Insert `list` between the current element and the next
    pub fn insert_list(&mut self, list: LinkedList<T>) {
        match (list.head, list.tail) {
            (Some(mut head), Some(mut tail)) => unsafe {
                head.as_mut().prev = self.current;
                tail.as_mut().next = self.next();
            },
            //splicing in an empty list should be a no-op
            (None, None) => return,
            _ => unreachable!(),
        }
        unsafe {
            match self.next() {
                None => self.list.tail = list.tail,
                Some(mut next) => next.as_mut().prev = list.tail,
            }
            match self.current {
                None => self.list.head = list.head,
                Some(mut prev) => prev.as_mut().next = list.head,
            }
        }
        self.list.len += list.len;
    }

    /// Insert `list` between the previous element and current
    pub fn insert_list_before(&mut self, list: LinkedList<T>) {
        match (list.head, list.tail) {
            (Some(mut head), Some(mut tail)) => unsafe {
                head.as_mut().prev = self.prev();
                tail.as_mut().next = self.current;
            },
            //splicing in an empty list should be a no-op
            (None, None) => return,
            _ => unreachable!(),
        }
        unsafe {
            match self.prev() {
                None => self.list.head = list.head,
                Some(mut next) => next.as_mut().next = list.head,
            }
            match self.current {
                None => self.list.tail = list.tail,
                Some(mut prev) => prev.as_mut().prev = list.tail,
            }
        }
        self.list.len += list.len;
        if self.current_len != 0 {
            self.current_len += list.len;
        }
    }

    /// Remove and return the item following the cursor
    pub fn pop(&mut self) -> Option<T> {
        self.next().map(|node| unsafe {
            self.list.len -= 1;
            self.current_len %= self.list.len + 1;

            let node = Box::from_raw(node.as_ptr());
            match self.current {
                None => self.list.head = node.next,
                Some(mut prev) => prev.as_mut().next = node.next,
            }
            match node.next {
                None => self.list.tail = self.current,
                Some(mut next) => {
                    next.as_mut().prev = self.current;
                }
            }
            Node::into_element(node)
        })
    }
    /// Remove and return the item before the cursor
    pub fn pop_prev(&mut self) -> Option<T> {
        self.prev().map(|node| unsafe {
            self.list.len -= 1;
            self.dec_len();

            let node = Box::from_raw(node.as_ptr());
            match node.prev {
                None => self.list.head = self.current,
                Some(mut prev) => prev.as_mut().next = self.current,
            }
            match self.current {
                None => self.list.tail = node.prev,
                Some(mut next) => next.as_mut().prev = node.prev,
            }
            Node::into_element(node)
        })
    }

    fn split_at(self, current: NonNull<Node<T>>, split_len: usize) -> LinkedList<T> {
        let total_len = self.list.len;

        let next = unsafe { (*current.as_ptr()).next };

        if let Some(next) = next {
            let new_head = Some(next);
            let new_tail = self.list.tail.take();
            let new_len = total_len - split_len;

            let old_head = self.list.head;
            let old_tail = Some(current);
            let old_len = total_len - new_len;

            unsafe {
                (*current.as_ptr()).next = None;
                (*next.as_ptr()).prev = None;
            }

            self.list.head = old_head;
            self.list.tail = old_tail;
            self.list.len = old_len;

            LinkedList {
                head: new_head,
                tail: new_tail,
                len: new_len,
                marker: PhantomData,
            }
        } else {
            LinkedList::new()
        }
    }

    /// Split the list in two after the current element
    /// The returned list consists of all elements following the current one.
    // note: consuming the cursor is not necessary here, but it makes sense
    // given the interface
    pub fn split(self) -> LinkedList<T> {
        use std::mem::replace;

        match self.current {
            None => replace(self.list, LinkedList::new()),

            Some(current) => {
                let split_len = self.current_len;
                self.split_at(current, split_len)
            }
        }
    }

    /// Split the list in two before the current element
    pub fn split_before(self) -> LinkedList<T> {
        use std::mem::replace;

        match self.current {
            None => replace(self.list, LinkedList::new()),
            Some(current) => match unsafe { (*current.as_ptr()).prev } {
                None => replace(self.list, LinkedList::new()),
                Some(prev) => {
                    let split_len = self.current_len - 1;
                    self.split_at(prev, split_len)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::iter::FromIterator;

    use super::{Cursor, CursorMut, LinkedList};

    fn mut_cmp_iterator<T, I>(list: &mut LinkedList<T>, iter: I)
    where
        T: PartialEq + Debug,
        I: IntoIterator<Item = T> + Clone + Iterator + DoubleEndedIterator<Item = T>,
    {
        {
            let mut cursor = list.cursor_mut();
            for i in iter.clone() {
                cursor.move_next();
                let mut i = i;
                assert_eq!(cursor.current(), Some(&mut i));
            }
            cursor.move_next();
            assert_eq!(cursor.current(), None);
        }
        {
            let mut cursor = list.cursor_mut();
            let iter = iter.rev();

            for i in iter {
                cursor.move_prev();
                let mut i = i;
                assert_eq!(cursor.current(), Some(&mut i));
            }
            cursor.move_prev();
            assert_eq!(cursor.current(), None);
        }
    }

    fn cmp_iterator<T, I>(list: &LinkedList<T>, iter: I)
    where
        T: PartialEq + Debug,
        I: IntoIterator<Item = T> + Clone + Iterator + DoubleEndedIterator<Item = T>,
    {
        {
            // test raw links
            let mut cursor = list.cursor();
            cursor.move_next();
            while let Some(current) = cursor.current {
                if let Some(next) = unsafe { (*current.as_ptr()).next } {
                    assert_eq!(unsafe { (*next.as_ptr()).prev }, Some(current));
                }
                cursor.move_next();
            }
        }
        {
            // test forwards iteration
            let mut cursor = list.cursor();
            for i in iter.clone() {
                cursor.move_next();
                assert_eq!(cursor.current(), Some(&i));
            }
            cursor.move_next();
            assert_eq!(cursor.current(), None);
        }
        {
            // test reverse iteration
            let mut cursor = list.cursor();
            let iter = iter.rev();

            for i in iter {
                cursor.move_prev();
                //println!("{:?}", cursor.current());
                assert_eq!(cursor.current(), Some(&i));
            }
            cursor.move_prev();
            assert_eq!(cursor.current(), None);
        }
    }

    #[test]
    fn sanity_test() {
        cmp_iterator(&LinkedList::from_iter(0..10), 0..10);
        mut_cmp_iterator(&mut LinkedList::from_iter(0..10), 0..10);
    }
    #[test]
    fn reverse() {
        let list = LinkedList::from_iter(0..4);
        let mut cursor = list.cursor();
        for i in (0..4).rev() {
            cursor.move_prev();
            assert_eq!(cursor.current(), Some(&i));
        }
        cursor.move_prev();
        assert_eq!(cursor.current(), None);
    }
    #[test]
    fn peek() {
        let list = LinkedList::from_iter(3..5);
        let cursor = list.cursor();
        assert_eq!(cursor.peek(), Some(&3));
        assert_eq!(cursor.peek_before(), Some(&4));
    }
    #[test]
    fn len() {
        let mut list = LinkedList::from_iter(0..5);
        assert_eq!(list.len, 5);
        let list2 = {
            let mut cursor = list.cursor_mut();
            cursor.move_next();
            cursor.move_next();
            cursor.split()
        };
        assert_eq!(list.len, 2);
        assert_eq!(list2.len, 3);
    }

    /*
        [Node:2] <- [List] ->   [Node:0]
            None  <- [Node:0] -> [Node:1]
        [Node:0] -> [Node:1] -> [Node:2]
        [Node:1] -> [Node:2] ->  None
    
        [Node:0] <- [List] ->   [Node:0]
            None  <- [Node:0] ->  None
    
        test cases:
            [L] cursor "points" to list:
                current = None (= List)
            [H] cursor has no prev:
                current = [Node:0]
            [T] cursor has no next:
                current = [Node:2]
            [G] general case:
                current = [Node:1]
    
            [S] single element, neither prev nor next
    
    */

    #[test]
    fn split() {
        fn test_split(n: usize, mut i: Option<usize>) {
            let mut list = LinkedList::from_iter(0..n);
            print!("split {:?} at {:?}", list, i);
            let tail = {
                let mut c = list.cursor_mut();
                if let Some(i) = i {
                    c.move_next();
                    for _ in 0..i {
                        c.move_next();
                    }
                }
                println!(" = {:?}", c.current());
                assert_eq!(i.as_mut(), c.current());
                c.split()
            };
            println!("old: {:?}", list);
            println!("new: {:?}", tail);
            match i {
                Some(i) => {
                    cmp_iterator(&list, 0..=i);
                    cmp_iterator(&tail, i + 1..n);
                }
                _ => {
                    cmp_iterator(&list, 0..0);
                    cmp_iterator(&tail, 0..n);
                }
            }
        }

        test_split(10, None); // case L
        test_split(10, Some(0)); // case H
        test_split(10, Some(9)); // case T
        test_split(10, Some(3)); // case G
        test_split(1, None); // case L
        test_split(1, Some(0)); // case S
    }
    #[test]
    fn split_before() {
        fn test_split(n: usize, i: Option<usize>) {
            let mut list = LinkedList::from_iter(0..n);
            print!("split {:?} before {:?}", list, i);
            let tail = {
                let mut c = list.cursor_mut();
                if let Some(i) = i {
                    c.move_next();
                    for _ in 0..i {
                        c.move_next();
                    }
                }
                println!(" = {:?}", c.current());
                c.split_before()
            };
            println!("old: {:?}", list);
            println!("new: {:?}", tail);
            match i {
                Some(i) => {
                    cmp_iterator(&list, 0..i);
                    cmp_iterator(&tail, i..n);
                }
                _ => {
                    cmp_iterator(&list, 0..0);
                    cmp_iterator(&tail, 0..n);
                }
            }
        }

        test_split(10, None); // case L
        test_split(10, Some(0)); // case L
        test_split(10, Some(1)); // case H
        test_split(10, Some(9)); // case T
        test_split(10, Some(3)); // case G
        test_split(1, None); // case L
        test_split(1, Some(0)); // case S
    }
}
