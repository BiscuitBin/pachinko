//// Time, as the double counts it. The specification types its fields
//// `Timestamp` and `Duration` and names no resolution; both are whole
//// milliseconds here, so `now + ttl` is exact (spec note G01).

/// A point in time: whole milliseconds since the clock's zero.
pub type Timestamp =
  Int

/// A span of time: whole milliseconds.
pub type Duration =
  Int

pub const millisecond: Duration = 1

pub const second: Duration = 1000

pub const minute: Duration = 60_000

pub const hour: Duration = 3_600_000
