// AUTO-GENERATED. DO NOT EDIT.
use crate::stack::char_array_stack::CharArrayStack;
use std::fmt;
use std::sync::Arc;

/// Immutable stack. Push/pop return new stacks (persistent data structure).
#[derive(Debug, Clone)]
pub struct ImmutableCharArrayStack {
    items: Arc<[char]>,
}

impl ImmutableCharArrayStack {
    pub fn from_mutable(stack: &CharArrayStack) -> Self {
        ImmutableCharArrayStack {
            items: Arc::from(stack.to_vec().into_boxed_slice()),
        }
    }
    pub fn of(values: &[char]) -> Self {
        ImmutableCharArrayStack {
            items: Arc::from(values),
        }
    }
    pub fn peek(&self) -> Option<char> {
        self.items.first().copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns (new stack without top, popped value).
    pub fn pop(&self) -> Option<(Self, char)> {
        if self.items.is_empty() {
            return None;
        }
        let val = self.items[0];
        let rest: Vec<char> = self.items[1..].to_vec();
        Some((
            ImmutableCharArrayStack {
                items: Arc::from(rest.into_boxed_slice()),
            },
            val,
        ))
    }

    /// Returns a new stack with the value pushed on top.
    pub fn push(&self, value: char) -> Self {
        let mut items = vec![value];
        items.extend_from_slice(&self.items);
        ImmutableCharArrayStack {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn to_mutable(&self) -> CharArrayStack {
        CharArrayStack::of(&self.items)
    }
    pub fn contains(&self, value: char) -> bool {
        self.items.contains(&value)
    }
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.items.iter().copied()
    }
    pub fn to_vec(&self) -> Vec<char> {
        self.items.to_vec()
    }
}

impl crate::traits::char_collection::CharCollection for ImmutableCharArrayStack {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: char) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.iter()
    }
}

impl crate::traits::char_collection::CharStack for ImmutableCharArrayStack {
    fn peek(&self) -> Option<char> {
        self.peek()
    }
}

impl fmt::Display for ImmutableCharArrayStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.items.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", v)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_peek_len() {
        let im = ImmutableCharArrayStack::of(&['a', 'b']);
        assert_eq!(im.len(), 2);
        assert_eq!(im.peek(), Some('a'));
    }
    #[test]
    fn test_push_immutable() {
        let im = ImmutableCharArrayStack::of(&['a']);
        let im2 = im.push('b');
        assert_eq!(im.len(), 1);
        assert_eq!(im2.len(), 2);
    }
    #[test]
    fn test_pop_immutable() {
        let im = ImmutableCharArrayStack::of(&['a', 'b']);
        let (im2, val) = im.pop().unwrap();
        assert_eq!(val, 'a');
        assert_eq!(im2.len(), 1);
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_to_mutable() {
        let im = ImmutableCharArrayStack::of(&['a']);
        let mut m = im.to_mutable();
        m.push('b');
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableCharArrayStack::of(&['a']).to_string().is_empty());
    }
}
