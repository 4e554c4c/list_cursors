#![allow(dead_code)]
#![feature(box_into_raw_non_null)]
#![feature(box_syntax)]
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

/// An Immutable look into a `LinkedList` that can be moved back and forth
pub struct Cursor<'list, T: 'list> {
    current: Option<NonNull<Node<T>>>,
    list: &'list LinkedList<T>,
}

impl<'list, T> Cursor<'list, T> {
    fn next(&self) -> Option<NonNull<Node<T>>> {
        self.current.map_or(self.list.head,|node| unsafe { node.as_ref().next })
    }
    fn prev(&self) -> Option<NonNull<Node<T>>> {
        self.current.map_or(self.list.tail,|node| unsafe { node.as_ref().prev })
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
        self.current.map_or(self.list.head,|node| unsafe { node.as_ref().next })
    }
    fn prev(&self) -> Option<NonNull<Node<T>>> {
        self.current.map_or(self.list.tail,|node| unsafe { node.as_ref().prev })
    }
    // `current_len` is in the range 0...self.list.len at all times
    fn inc_len(&mut self) {
        self.current_len += 1;
        self.current_len %= self.list.len+1;
    }
    fn dec_len(&mut self) {
        self.current_len += self.list.len;
        self.current_len %= self.list.len+1;
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
            (Some(mut head),Some(mut tail)) => {
                unsafe {
                    head.as_mut().prev = self.current;
                    tail.as_mut().next = self.next();
        
                }
            },
            //splicing in an empty list should be a no-op
            (None,None) => return,
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
            (Some(mut head),Some(mut tail)) => {
                unsafe {
                    head.as_mut().prev = self.prev();
                    tail.as_mut().next = self.current;

                }
            },
            //splicing in an empty list should be a no-op
            (None,None) => return,
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
            self.current_len %= self.list.len+1;

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

    // TODO fix splits

    /// Split the list in two after the current element
    /// The returned list consists of all elements following the current one.
    // note: consuming the cursor is not necessary here, but it makes sense
    // given the interface
    pub fn split(self) -> LinkedList<T> {
        self.next().map(|node| {
            match self.current {
                None => self.list.head = None,
                Some(mut last) => unsafe {
                    last.as_mut().next = None
                },
            }
            let old_tail = self.list.tail;
            self.list.tail = self.current;

            let old_len = self.list.len;
            self.list.len = self.current_len;
            LinkedList {
                head: Some(node),
                tail: old_tail,
                len: old_len - self.current_len,
            }
        }).unwrap_or_else(LinkedList::new)
    }
    /// Split the list in two before the current element
    pub fn split_before(self) -> LinkedList<T> {
        match self.prev() {
            None => std::mem::replace(self.list, LinkedList::new()),
            Some(mut prev) => {
                self.current.map(|current| unsafe {
                    let old_tail = self.list.tail;
                    self.list.tail = Some(prev);
                    let old_len = self.list.len;
                    self.list.len = self.current_len -1;;

                    LinkedList {
                        head: Some(current),
                        tail: old_tail,
                        len: old_len - self.current_len,
                    }
                }).unwrap_or_else(LinkedList::new)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LinkedList,Cursor,CursorMut};
    fn from_iter<T,I: IntoIterator<Item=T>>(iter: I) -> LinkedList<T>
    {
        let mut list = LinkedList::new();
        {
            let mut cursor = list.cursor_mut();
            for el in iter {
                cursor.insert_before(el);
            }
        }
        list
    }
    use std::fmt::Debug;
    fn cmp_iterator<T, I>(list: LinkedList<T>, iter: I)
        where T: PartialEq + Debug, I: IntoIterator<Item=T>
    {
        let mut cursor = list.cursor();
        for i in iter {
            cursor.move_next();
            assert_eq!(cursor.current(), Some(&i));
        }
        cursor.move_next();
        assert_eq!(cursor.current(),None);
    }
    fn print_list<T: Debug>(list: &LinkedList<T>) {
        let mut cursor = list.cursor();
        cursor.move_next();
        print!("[");
        loop {
            match cursor.current() {
                Some(i) => print!("{:?},",i),
                None => break,
            }
            cursor.move_next();
        }
        println!("]")
    }

    #[test]
    fn sanity_test() {
        cmp_iterator(from_iter(0..10),0..10);
    }
    #[test]
    fn reverse() {
        let list = from_iter(0..4);
        let mut cursor = list.cursor();
        for i in (0..4).rev() {
            cursor.move_prev();
            assert_eq!(cursor.current(), Some(&i));
        }
        cursor.move_prev();
        assert_eq!(cursor.current(),None);
    }
    #[test]
    fn peek() {
        let list = from_iter(3..5);
        let cursor = list.cursor();
        assert_eq!(cursor.peek(), Some(&3));
        assert_eq!(cursor.peek_before(), Some(&4));
    }
    #[test]
    fn len() {
        let mut list = from_iter(0..5);
        assert_eq!(list.len,5);
        let list2 = {
            let mut cursor = list.cursor_mut();
            cursor.move_next();
            cursor.move_next();
            cursor.split()
        };
        assert_eq!(list.len, 2);
        assert_eq!(list2.len,3);
    }
    #[test]
    fn split() {
        let mut list = from_iter(0..10);
        let list2 = {
            let mut cursor = list.cursor_mut();
            cursor.move_next();
            cursor.move_next();
            cursor.move_next();
            cursor.split()
        };
        print_list(&list);
        print_list(&list2);
        cmp_iterator(list,0..3);
        cmp_iterator(list2,3..10);
    }
    #[test]
    fn split_before() {
        let mut list = from_iter(0..10);
        let list2 = {
            let mut cursor = list.cursor_mut();
            cursor.move_next();
            cursor.move_next();
            cursor.move_next();
            cursor.move_next();
            cursor.split_before()
        };
        print_list(&list);
        print_list(&list2);
        cmp_iterator(list,0..3);
        cmp_iterator(list2,3..10);
    }
}
