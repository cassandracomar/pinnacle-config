use std::{collections::VecDeque, convert::identity, fmt::Display};

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

impl<T: Display> Display for Zipper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for t in self.iter() {
            write!(f, " {t},")?;
        }
        write!(f, "]")
    }
}

/// the direction to move in, relative to the original order of the elements in the source sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceDirection {
    Original,
    Reverse,
}

fn push_and_yield<T>(n: &mut VecDeque<T>, t: T) -> &mut VecDeque<T> {
    n.push_front(t);
    n
}

fn push_back_and_yield<T>(n: &mut VecDeque<T>, t: T) -> &mut VecDeque<T> {
    n.push_back(t);
    n
}

fn reset<T>(nl: &mut VecDeque<T>, pl: &mut VecDeque<T>) {
    pl.drain(..).fold(nl, push_and_yield);
}

fn pop_push<T>(pop: &mut VecDeque<T>, push: &mut VecDeque<T>) {
    push.push_front(pop.pop_front().unwrap())
}

impl<T> Zipper<T> {
    /// construct an empty `Zipper`
    pub fn new() -> Self {
        Self {
            forward: VecDeque::new(),
            backward: VecDeque::new(),
        }
    }

    /// move the focus to the next element in the sequence, in the provided direction. this
    /// function rotates back to the start of the sequence when `circle_step` is called on
    /// the last element of the sequence.
    pub fn circle_step(self, dir: SequenceDirection) -> Self {
        if self.size() == 0 {
            return self;
        }

        match dir {
            SequenceDirection::Original => self.advance_focus(dir).rotate_stacks(dir),
            SequenceDirection::Reverse => self.rotate_stacks(dir).advance_focus(dir),
        }
    }

    /// get the number of elements in the `Zipper`
    pub fn size(&self) -> usize {
        self.forward.len() + self.backward.len()
    }

    /// skip ahead in the sequence until we reach the first element that satisfies the provided predicate.
    /// because `Zipper::circle_step` circularizes the `Zipper`, we will eventually find the requested element.
    /// this moves the `Zipper`'s focus to the requested element.
    pub fn refocus(self, mut p: impl FnMut(&T) -> bool) -> Self {
        let f = move |s: Self, _| {
            let s = s.circle_step(SequenceDirection::Original);
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

    /// reset the focused element to the start of the original sequence
    pub fn reset_start(mut self) -> Self {
        reset(&mut self.forward, &mut self.backward);
        self
    }

    /// reset the focused element to the end of the original sequence. note that the caller needs to advance the zipper
    /// one step in reverse to actually maintain the zipper invariant, that an element is always focused.
    fn reset_end_impl(mut self) -> Self {
        reset(&mut self.backward, &mut self.forward);
        self
    }

    /// reset the focused element to the end of the original sequence.
    pub fn reset_end(self) -> Self {
        self.reset_end_impl()
            .circle_step(SequenceDirection::Reverse)
    }

    /// take one step in the requested direction. this pops an element from the stack matching the direction of motion
    /// and pushes it onto the reverse stacks.
    fn advance_focus(mut self, dir: SequenceDirection) -> Self {
        match dir {
            SequenceDirection::Original => pop_push(&mut self.forward, &mut self.backward),
            SequenceDirection::Reverse => pop_push(&mut self.backward, &mut self.forward),
        };

        self
    }

    /// rotate the stack counter to the direction of motion into the stack matching the direction of motion, if necessary.
    /// this rotation is only required when the stack matching the direction of motion has run out of elements. we thus
    /// circularize the `Zipper`, ensuring that we always have a next element in the appropriate direction, so long as the
    /// `Zipper` itself is not empty.
    fn rotate_stacks(self, dir: SequenceDirection) -> Self {
        match dir {
            SequenceDirection::Original if self.forward.is_empty() => self.reset_start(),
            SequenceDirection::Reverse if self.backward.is_empty() => self.reset_end_impl(),
            _ => self,
        }
    }

    /// retrieve the element focused by the `Zipper`
    pub fn focus(&self) -> Option<&T> {
        self.forward.front()
    }

    /// yield an `Iterator` that iterates in the order imposed by the original sequence but starting at the currently
    /// focused element. the element following the last element of the original sequence is the first element of the
    /// sequence.
    pub fn iter(&'_ self) -> ZipperIter<'_, T> {
        ZipperIter {
            zipper: self,
            count: self.size(),
            cursor: 0,
            dir: SequenceDirection::Original,
        }
    }

    /// yield an `Iterator` that iterates in reverse over the original sequence but starting at the currently
    /// focused element. the element following the first element of the original sequence is the last element of the
    /// sequence.
    pub fn reverse_iter(&'_ self) -> ZipperIter<'_, T> {
        ZipperIter {
            zipper: self,
            count: self.size(),
            cursor: -1,
            dir: SequenceDirection::Reverse,
        }
    }
}

impl<T> FromIterator<T> for Zipper<T> {
    fn from_iter<U: IntoIterator<Item = T>>(iter: U) -> Self {
        let mut s = Self::new();
        iter.into_iter().fold(&mut s.forward, push_back_and_yield);
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
    cursor: isize,
    /// this iterator can go forwards or backwards
    dir: SequenceDirection,
}

impl<'a, T> Iterator for ZipperIter<'a, T>
where
    T: 'a,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        // this really highlights the advantage of the zipper approach. the zipper is structurally correct.
        // meanwhile, this is a mess of asymmetry and magic numbers.
        let (nl, pl, cond, f): (_, _, _, Box<dyn Fn(isize) -> usize>) = match self.dir {
            SequenceDirection::Original => (
                &self.zipper.forward,
                &self.zipper.backward,
                self.cursor < self.count as isize,
                Box::new(|c| c as usize),
            ),
            SequenceDirection::Reverse => (
                &self.zipper.backward,
                &self.zipper.forward,
                self.cursor < self.count as isize - 1,
                Box::new(|c| {
                    if c < 0 {
                        (c + self.count as isize) as usize
                    } else {
                        c as usize
                    }
                }),
            ),
        };

        if cond {
            let nl_len = nl.len();
            let pl_len = pl.len();

            let res = if f(self.cursor) < nl_len {
                nl.get(f(self.cursor))
            } else {
                // when we reach the bottom of `nl`, the next element in the sequence is the *last* element of `pl`
                pl.get(pl_len - (f(self.cursor) - nl_len + 1))
            };

            self.cursor += 1;

            res
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Rem;

    use super::*;

    #[test]
    fn reset_start_and_end_should_reset() {
        let zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        let zipper = zipper.reset_end();
        assert_eq!(
            zipper.focus().copied(),
            Some(9),
            "resetting a zipper to the end should focus the last element"
        );

        let zipper = zipper.reset_end();
        assert_eq!(
            zipper.focus().copied(),
            Some(9),
            "resetting to the end when focusing the end should be idempotent"
        );

        let zipper = zipper.reset_start();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "resetting a zipper to the start should focus the first element"
        );

        let zipper = zipper.reset_start();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "resetting to the start when focusing the start should be idempotent"
        );
    }

    #[test]
    fn cycle_step_forward_moves_focus_forward() {
        let zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        let zipper = zipper.circle_step(SequenceDirection::Original);
        assert_eq!(
            zipper.focus().copied(),
            Some(1),
            "stepping forward should advance to the second element"
        );

        let zipper = zipper.reset_end().circle_step(SequenceDirection::Original);
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "moving to the end and advancing should circle back to the start"
        );
    }

    #[test]
    fn cycle_step_backward_moves_focus_backward() {
        let zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        let zipper = zipper.circle_step(SequenceDirection::Reverse);
        assert_eq!(
            zipper.focus().copied(),
            Some(9),
            "stepping backward from the first element should advance to the last element"
        );

        let zipper = zipper.reset_end().circle_step(SequenceDirection::Reverse);
        assert_eq!(
            zipper.focus().copied(),
            Some(8),
            "moving to the end and stepping back should yield the second-to-last element"
        );
    }

    #[test]
    fn refocus_should_focus_the_first_matching_element() {
        let zipper = (0..10).into_iter().collect::<Zipper<_>>();
        assert_eq!(
            zipper.focus().copied(),
            Some(0),
            "a newly collected zipper should focus the first element"
        );

        let zipper = zipper.refocus(|t| *t == 5);
        assert_eq!(
            zipper.focus().copied(),
            Some(5),
            "refocus should focus the selected element"
        );

        let zipper = zipper.refocus(|t| t.rem(3) == 0);
        assert_eq!(
            zipper.focus().copied(),
            Some(6),
            "refocus should focus the first element satisfying the predicate"
        );
    }

    #[test]
    fn iterator_yields_all_elements_starting_from_focus() {
        let zipper = (0..10)
            .into_iter()
            .collect::<Zipper<_>>()
            .refocus(|t| *t == 5);
        assert_eq!(
            zipper.focus().copied(),
            Some(5),
            "refocus should focus the selected element"
        );

        let v = zipper.iter().copied().collect::<Vec<_>>();
        assert_eq!(
            &v,
            &[5, 6, 7, 8, 9, 0, 1, 2, 3, 4],
            "iterator should produce all elements in order, starting from the focus"
        );

        let v = zipper.reverse_iter().copied().collect::<Vec<_>>();
        assert_eq!(
            &v,
            &[5, 4, 3, 2, 1, 0, 9, 8, 7, 6],
            "reverse iterator should produce all elements in reverse order, starting from the focus"
        );

        let s = zipper.to_string();
        assert_eq!(
            &s, "[ 5, 6, 7, 8, 9, 0, 1, 2, 3, 4,]",
            "pretty printing the zipper should work"
        );
    }
}
