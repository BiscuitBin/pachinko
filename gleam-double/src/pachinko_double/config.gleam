//// The specification's `config` block. A `Config` can only be made by `new`,
//// which refuses values that break a config invariant, so an owner cannot be
//// started with one (spec note G02).

import gleam/list
import pachinko_double/refusals.{type Refusal, ConfigRefused}
import pachinko_double/time.{type Duration}

/// The five values, as plain fields. `new` turns them into a `Config`.
pub type Settings {
  Settings(
    lease_ttl: Duration,
    max_consecutive_expiries: Int,
    max_pull_batch: Int,
    superseded_ttl: Duration,
    dead_letter_ttl: Duration,
  )
}

/// Settings that passed `new`.
pub opaque type Config {
  Config(settings: Settings)
}

/// The specification's five defaults.
pub fn defaults() -> Settings {
  Settings(
    lease_ttl: 5 * time.minute,
    max_consecutive_expiries: 3,
    max_pull_batch: 100,
    superseded_ttl: 12 * time.hour,
    dead_letter_ttl: 12 * time.hour,
  )
}

/// The default configuration.
pub fn default() -> Config {
  Config(defaults())
}

/// Makes a configuration, or refuses the settings, naming every config
/// invariant they break, in the specification's order. The boundary values
/// are admitted: 1 for the two limits and 0 for each TTL.
pub fn new(settings: Settings) -> Result(Config, Refusal) {
  case broken(settings) {
    [] -> Ok(Config(settings))
    invariants -> Error(ConfigRefused(invariants))
  }
}

/// The names of the config invariants the settings break.
fn broken(settings: Settings) -> List(String) {
  [
    #("ExpiryLimitPositive", settings.max_consecutive_expiries >= 1),
    #("PullBatchPositive", settings.max_pull_batch >= 1),
    #("LeaseTtlNonNegative", settings.lease_ttl >= 0),
    #("SupersededTtlNonNegative", settings.superseded_ttl >= 0),
    #("DeadLetterTtlNonNegative", settings.dead_letter_ttl >= 0),
  ]
  |> list.filter(fn(invariant) { !invariant.1 })
  |> list.map(fn(invariant) { invariant.0 })
}

/// The values a configuration holds.
pub fn settings(config: Config) -> Settings {
  config.settings
}
