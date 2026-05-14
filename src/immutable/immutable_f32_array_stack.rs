// AUTO-GENERATED. DO NOT EDIT.
use crate::stack::f32_array_stack::F32ArrayStack;
use std::fmt;
use std::sync::Arc;

/// Immutable stack. Push/pop return new stacks (persistent data structure).
#[derive(Debug, Clone)]
pub struct ImmutableF32ArrayStack {
    items: Arc<[f32]>,
}

impl ImmutableF32ArrayStack {
    pub fn from_mutable(stack: &F32ArrayStack) -> Self {
        ImmutableF32ArrayStack {
            items: Arc::from(stack.to_vec().into_boxed_slice()),
        }
    }
    pub fn of(values: &[f32]) -> Self {
        ImmutableF32ArrayStack {
            items: Arc::from(values),
        }
    }
    pub fn peek(&self) -> Option<f32> {
        self.items.first().copied()
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns (new stack without top, popped value).
    pub fn pop(&self) -> Option<(Self, f32)> {
        if self.items.is_empty() {
            return None;
        }
        let val = self.items[0];
        let rest: Vec<f32> = self.items[1..].to_vec();
        Some((
            ImmutableF32ArrayStack {
                items: Arc::from(rest.into_boxed_slice()),
            },
            val,
        ))
    }

    /// Returns a new stack with the value pushed on top.
    pub fn push(&self, value: f32) -> Self {
        let mut items = vec![value];
        items.extend_from_slice(&self.items);
        ImmutableF32ArrayStack {
            items: Arc::from(items.into_boxed_slice()),
        }
    }

    pub fn to_mutable(&self) -> F32ArrayStack {
        F32ArrayStack::of(&self.items)
    }
    pub fn contains(&self, value: f32) -> bool {
        self.items.iter().any(|&v| v.to_bits() == value.to_bits())
    }
    pub fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        self.items.iter().copied()
    }
    pub fn to_vec(&self) -> Vec<f32> {
        self.items.to_vec()
    }
}

impl crate::traits::f32_collection::F32Collection for ImmutableF32ArrayStack {
    fn len(&self) -> usize {
        self.len()
    }
    fn contains(&self, value: f32) -> bool {
        self.contains(value)
    }
    fn iter(&self) -> impl Iterator<Item = f32> + '_ {
        self.iter()
    }
}

impl crate::traits::f32_collection::F32Stack for ImmutableF32ArrayStack {
    fn peek(&self) -> Option<f32> {
        self.peek()
    }
}

impl fmt::Display for ImmutableF32ArrayStack {
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
        let im = ImmutableF32ArrayStack::of(&[1.0f32, 2.0f32]);
        assert_eq!(im.len(), 2);
        assert_eq!(im.peek(), Some(1.0f32));
    }
    #[test]
    fn test_push_immutable() {
        let im = ImmutableF32ArrayStack::of(&[1.0f32]);
        let im2 = im.push(2.0f32);
        assert_eq!(im.len(), 1);
        assert_eq!(im2.len(), 2);
    }
    #[test]
    fn test_pop_immutable() {
        let im = ImmutableF32ArrayStack::of(&[1.0f32, 2.0f32]);
        let (im2, val) = im.pop().unwrap();
        assert_eq!(val, 1.0f32);
        assert_eq!(im2.len(), 1);
        assert_eq!(im.len(), 2);
    }
    #[test]
    fn test_to_mutable() {
        let im = ImmutableF32ArrayStack::of(&[1.0f32]);
        let mut m = im.to_mutable();
        m.push(2.0f32);
        assert_eq!(im.len(), 1);
    }
    #[test]
    fn test_display() {
        assert!(!ImmutableF32ArrayStack::of(&[1.0f32]).to_string().is_empty());
    }
}
