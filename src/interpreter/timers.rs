//! `every N seconds:` / `after N seconds:` timer scheduling and firing.

use super::{Flow, Interpreter};
use crate::error::AzraError;

/// Tolerance for accumulated floating-point frame steps (e.g. `60 * (1.0 / 60.0)`).
const TIMER_EPSILON: f64 = 1e-9;

impl Interpreter {
    /// Fire every timer whose `next_due` has reached the script clock.
    /// Called at the end of [`Interpreter::tick`].
    pub(super) fn fire_due_timers(&mut self) -> Result<(), AzraError> {
        let taken = std::mem::take(&mut self.timers);
        let mut rest = taken.into_iter();
        while let Some(mut timer) = rest.next() {
            if timer.next_due > self.now + TIMER_EPSILON {
                self.timers.push(timer);
                continue;
            }
            if timer.repeating {
                timer.next_due += timer.interval;
                // Coalesce missed fires (a long frame must not burst).
                if timer.next_due <= self.now + TIMER_EPSILON {
                    timer.next_due = self.now + timer.interval;
                }
                self.timers.push(timer.clone());
            }
            let saved = self.scope.clone();
            self.scope = timer.closure.clone();
            let result = self.exec_block(&timer.body);
            self.scope = saved;
            match result {
                Ok(Flow::Normal) => {}
                Ok(Flow::Return(_)) => {
                    self.timers.extend(rest);
                    return Err(AzraError::runtime_at(
                        "`give back` outside of a function (in a timer)",
                        timer.line,
                    ));
                }
                Ok(Flow::Break) | Ok(Flow::Continue) => {
                    self.timers.extend(rest);
                    return Err(AzraError::runtime_at(
                        "`stop loop` / `skip next` outside of a loop (in a timer)",
                        timer.line,
                    ));
                }
                Err(e) => {
                    // Keep the remaining timers alive for the next tick.
                    self.timers.extend(rest);
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}
