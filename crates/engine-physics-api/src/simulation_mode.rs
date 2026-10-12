//! Domain configuration names independent of managed and native ABI encodings.

/// Selected scheduling policy; Fixed does not imply an implemented scheduler.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SimulationMode {
    /// Configuration captured/restored by the measured fixture.
    #[default]
    Fixed,
    /// Explicit manual stepping; runtime admission owns this policy.
    Script,
}

/// The bounded backend's supported body interpolation policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BodyInterpolation {
    /// Publish the simulated position without interpolation.
    #[default]
    None,
}
