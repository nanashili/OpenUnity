//! Callback sequencing shared by untimed scene and script commands.

use engine_scripting_bridge::ScriptSession;
use std::error::Error;

pub(super) fn execute(
    mut session: ScriptSession<'_>,
    initial: i64,
    ticks: &[i64],
    shutdown: i64,
) -> Result<Vec<i64>, Box<dyn Error>> {
    let mut outputs = vec![session.initialize(initial)?];
    for tick in ticks {
        outputs.push(session.tick(*tick)?);
    }
    outputs.push(session.shutdown(shutdown)?);
    session.close()?;

    Ok(outputs)
}
