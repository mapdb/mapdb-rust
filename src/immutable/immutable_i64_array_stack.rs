// AUTO-GENERATED. DO NOT EDIT.
use crate::stack::i64_array_stack::I64ArrayStack;
use std::fmt;
use std::sync::Arc;

/// Immutable stack. Push/pop return new stacks (persistent data structure).
#[derive(Debug, Clone)]
pub struct ImmutableI64ArrayStack {
    items: Arc<[i64]>,
}

impl ImmutableI64ArrayStack {
    pub fn from_mutable(stack: &I64ArrayStack) -> Self {
        ImmutableI64ArrayStack {
            items: Arc::from(stack.to_vec().into_boxed_slice()),
        }
    }
    pub fn of(values: &[i64]) -> Self {
        ImmutableI64ArrayStack {
            items: Arc::from(values),
        }
    }
    pub fn peek(&self) -> Option<i64> {
        self.items.first().copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns (new stack without top, popped value).
    pub fn pop(&self) -> Option<(Self, i64)> {
        if self.items.is_empty() {
            return None;
        }
        let val = self.items[0];
        let rest: Vec<i64> = self.items[1..].to_vec();
        Some((
            ImmutableI64ArrayStack {
                items: Arc::from(rest.into_boxed_slice()),
            },
            val,
        ))
    }

    /// Returns a new stack with the value pushed on top.
    pub fn push(&self, value: i64) -> Self {
        let mut items = vec![value];
        items.extend_from_slice(&self.items);
        ImmutableI64ArrayStack {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn to_mutable(&self) -> I64ArrayStack {
        I64ArrayStack::of(&self.items)
    }
    pub fn contains(&self, value: i64) -> bool {
        self.items.contains(&value)
    }
    pub fn iter(&self) -> impl Iterator<Item = i64> + '_ {
        self.items.iter().copied()
    }
    pub fn to_vec(&self) -> Vec<i64> {
        self.items.to_vec()
    }
}

impl crate::traits::i64_collection::I64Collection for ImmutableI64ArrayStack {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: i64) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = i64> + '_ {
        self.iter()
    }
}

impl crate::traits::i64_collection::I64Stack for ImmutableI64ArrayStack {
    fn peek(&self) -> Option<i64> {
        self.peek()
    }
}

impl fmt::Display for ImmutableI64ArrayStack {
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
        let im = ImmutableI64ArrayStack::of(&[1, 2]);
        assert_eq!(im.len(), 2);
        assert_eq!(im.peek(), Some(1));
    }
    #[test]
    fn test_push_immutable() {
        let im = ImmutableI64ArrayStack::of(&[1]);
        let im2 = im.push(2);
        assert_eq!(im.len(), 1);
        assert_eq!(im2.len(), 2);
    }
    #[test]
    fn test_pop_immutable() {
        let im = ImmutableI64ArrayStack::of(&[1, 2]);
        let (im2, val) = im.pop().unwrap();
        assert_eq!(val, 1);
        assert_eq!(im2.len(), 1);
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_to_mutable() {
        let im = ImmutableI64ArrayStack::of(&[1]);
        let mut m = im.to_mutable();
        m.push(2);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableI64ArrayStack::of(&[1]).to_string().is_empty());
    }
}
