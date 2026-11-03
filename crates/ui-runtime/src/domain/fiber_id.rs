//! The identity of a fiber.

/// Identifies one mounted component's fiber. The runtime allocates ids in
/// increasing order.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct FiberId(u64);

impl FiberId {
    /// Wrap a raw fiber id, such as one a debug client sends back.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// The raw fiber id.
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests;
