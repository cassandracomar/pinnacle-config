use std::{collections::VecDeque, convert::identity};

/// the classic functional data structure. it's like an iterator except that it needs to allocate.
/// opening a `Zipper` over a sequence allows moving both forwards and backwards through it.
/// a `Zipper` can be opened over any sequenceable data -- e.g. a traversal over a tree works
/// just as well as a list.
///
/// `Iterator`s are usually more efficient because they can avoid allocating buffers to store references
/// to the elements of the sequence -- in the worst case, the `Zipper` is allocating sufficient space for
/// twice the length of the sequence. however, `Zipper`s are substantially more flexible, allowing iteration
/// in both directions.
///
/// this particular `Zipper` has been turned into a ring. when the last element of the sequence is reached,
/// the next element is the first element of the sequence (and vice versa when iterating in reverse).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Zipper<T> {
    /// a stack for elements occurring later in the sequence.
    /// the first element of this stack is the one currently focused.
    forward: VecDeque<T>,
    /// a stack for elements occurring earlier in the sequence
    backward: VecDeque<T>,
}

/// the direction to move in relative to the original order of the elements in the source sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceDirection {
    Next,
    Previous,
}

fn push_and_yield<T>(n: &mut VecDeque<T>, t: T) -> &mut VecDeque<T> {
    n.push_front(t);
    n
}

fn reset<T>(nl: &mut VecDeque<T>, pl: &mut VecDeque<T>) {
    pl.drain(..).fold(nl, push_and_yield);
}

fn pop_push<T>(nl: &mut VecDeque<T>, pl: &mut VecDeque<T>) {
    pl.push_front(nl.pop_front().unwrap())
}

impl<T> Zipper<T> {
    pub fn new() -> Self {
        Self {
            forward: VecDeque::new(),
            backward: VecDeque::new(),
        }
    }

    /// move the focus to the next element in the sequence, in the provided direction. this
    /// function rotates back to the start of the sequence when `next_in_dir` is
    /// called on the last element of the sequence.
    pub fn circle_step(self, dir: SequenceDirection) -> Self {
        if self.size() == 0 {
            return self;
        }

        match dir {
            SequenceDirection::Next => self.advance_focus(dir).rotate_stacks(dir),
            SequenceDirection::Previous => self.rotate_stacks(dir).advance_focus(dir),
        }
    }

    pub fn size(&self) -> usize {
        self.forward.len() + self.backward.len()
    }

    /// skip ahead in the sequence until we reach the first element that satisfies the provided predicate.
    /// because `Zipper::next_in_dir` circularizes the `Zipper`, we will eventually find the requested element.
    /// this moves the `Zipper`'s focus to the requested element.
    pub fn refocus(self, mut p: impl FnMut(&T) -> bool) -> Self {
        let f = move |s: Self, _| {
            let s = s.circle_step(SequenceDirection::Next);
            match s.focus() {
                Some(t) if !p(t) => Ok(s),
                _ => Err(s), // we've found the focused window so break
            }
        };

        (0..self.size()) // only check each element once
            .into_iter()
            .try_fold(self, f)
            .unwrap_or_else(identity)
    }

    pub fn reset_start(mut self) -> Self {
        reset(&mut self.forward, &mut self.backward);
        self
    }

    pub fn reset_end(mut self) -> Self {
        reset(&mut self.backward, &mut self.forward);
        self
    }

    /// take one step in the requested direction. this pops an element from the stack matching the direction of motion
    /// and pushes it onto the reverse stacks.
    fn advance_focus(mut self, dir: SequenceDirection) -> Self {
        match dir {
            SequenceDirection::Next => pop_push(&mut self.forward, &mut self.backward),
            SequenceDirection::Previous => pop_push(&mut self.backward, &mut self.forward),
        };

        self
    }

    /// rotate the stack counter to the direction of motion into the stack matching the direction of motion, if necessary.
    /// this rotation is only required when the stack matching the direction of motion has run out of elements. we thus
    /// circularize the `Zipper`, ensuring that we always have a next element in the appropriate direction, so long as the
    /// `Zipper` itself is not empty.
    fn rotate_stacks(self, dir: SequenceDirection) -> Self {
        match dir {
            SequenceDirection::Next if self.forward.is_empty() => self.reset_start(),
            SequenceDirection::Previous if self.backward.is_empty() => self.reset_end(),
            _ => self,
        }
    }

    /// retrieve the element focused by the `Zipper`
    pub fn focus(&self) -> Option<&T> {
        self.forward.front()
    }

    pub fn iter(&'_ self) -> ZipperIter<'_, T> {
        ZipperIter {
            zipper: self,
            count: self.size(),
            cursor: 0,
            dir: SequenceDirection::Next,
        }
    }

    pub fn reverse_iter(&'_ self) -> ZipperIter<'_, T> {
        ZipperIter {
            zipper: self,
            count: self.size(),
            cursor: 0,
            dir: SequenceDirection::Previous,
        }
    }
}

impl<T> FromIterator<T> for Zipper<T> {
    fn from_iter<U: IntoIterator<Item = T>>(iter: U) -> Self {
        let mut s = Self::new();
        iter.into_iter().fold(&mut s.forward, push_and_yield);
        s
    }
}

/// an `Iterator` that yields the elements of the sequence the `Zipper` was opened over,
/// starting with the currently focused element and continuing until all elements have been yielded.
/// sequence ordering is preserved.
pub struct ZipperIter<'a, T> {
    /// sequence state
    zipper: &'a Zipper<T>,
    /// number of elements in the sequence
    count: usize,
    /// keep track of which items in the sequence we've already yielded -- otherwise we'll spin indefinitely.
    cursor: usize,
    /// this iterator can go forwards or backwards
    dir: SequenceDirection,
}

impl<'a, T> Iterator for ZipperIter<'a, T>
where
    T: 'a,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let (nl, pl) = match self.dir {
            SequenceDirection::Next => (&self.zipper.forward, &self.zipper.backward),
            SequenceDirection::Previous => (&self.zipper.backward, &self.zipper.forward),
        };

        if self.cursor < self.count {
            let nl_len = nl.len();
            let pl_len = pl.len();

            let res = if self.cursor < nl_len {
                nl.get(self.cursor)
            } else {
                // when we reach the bottom of `nl`, the next element in the sequence is the *last* element of pl
                pl.get(pl_len - (self.cursor - nl_len + 1))
            };
            self.cursor += 1;

            res
        } else {
            None
        }
    }
}
