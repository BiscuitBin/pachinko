//// How an outcome fails, as a value: a refusal, a guard overlap, a defect
//// or a broken invariant. `refusal` is the domain's refusal type,
//// `refusals.Refusal` for this queue.

/// Every way an outcome fails. Each is a value: nothing here panics.
pub type Failure(refusal) {
  /// The action was refused and changed nothing.
  Refused(refusal: refusal)
  /// More than one guard of a table held, which the specification says
  /// cannot happen: its guards are disjoint.
  GuardOverlap(trigger: String, rules: List(String))
  /// The double's own rules are wrong, and `cause` says how. `trigger` is the
  /// table the outcome had reached.
  Defect(trigger: String, cause: Cause)
  /// The outcome left the state breaking these invariants.
  BrokenInvariant(invariants: List(String))
}

/// Why an outcome is a defect.
pub type Cause {
  /// The trigger reached a `Defective` table, and none of its rules held.
  NoRuleHeld
  /// The effect of `rule` did not find what its guard promised.
  EffectFailed(rule: String, reason: String)
}
