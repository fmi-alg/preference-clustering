use metered::clear::Clear;
use metered::metric::{Advice, Enter, Metric, OnResult};
use serde::Serialize;

use parking_lot::Mutex;
use std::{
    fmt::{self, Display},
    time::{Duration, Instant},
};

#[derive(Debug, Serialize, Default, Clone)]
pub struct AggregateTimer {
    dur: Duration,
    count: usize,
}

impl AggregateTimer {
    pub fn add_measurement(&mut self, m: Duration) {
        self.dur += m;
        self.count += 1;
    }

    pub fn dur(&self) -> Duration {
        self.dur
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

impl Display for AggregateTimer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.dur.as_secs_f64();
        write!(
            f,
            "{:>5} measurements took {:.3} s ({:.3} ms per measurement)",
            self.count,
            secs,
            secs * 1000.0 / self.count as f64
        )
    }
}

impl std::ops::Add<&AggregateTimer> for &AggregateTimer {
    type Output = AggregateTimer;

    fn add(self, rhs: &AggregateTimer) -> Self::Output {
        let dur = self.dur + rhs.dur;
        let count = self.count + rhs.count;
        AggregateTimer { dur, count }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct SimpleTime {
    timer: Mutex<AggregateTimer>,
}

impl Display for SimpleTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let timer = self.timer.lock();
        write!(f, "{}", timer)
    }
}

impl<R> Metric<R> for SimpleTime {}

impl Enter for SimpleTime {
    type E = Instant;
    fn enter(&self) -> Self::E {
        Self::E::now()
    }
}

impl<R> OnResult<R> for SimpleTime {
    fn leave_scope(&self, enter: Self::E) -> Advice {
        let time = enter.elapsed();
        self.timer.lock().add_measurement(time);
        Advice::Return
    }
}

impl SimpleTime {
    pub fn timer(&self) -> AggregateTimer {
        (*self.timer.lock()).clone()
    }
}

impl Clear for SimpleTime {
    fn clear(&self) {
        *self.timer.lock() = AggregateTimer::default();
    }
}

#[derive(Debug, Serialize, Default)]
pub struct YesNoTime {
    yes: Mutex<AggregateTimer>,
    no: Mutex<AggregateTimer>,
}

impl Display for YesNoTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let yes = self.yes.lock();
        let no = self.no.lock();

        // The &* is needed to first dereference from the MutexGuard and then borrow for the add ...
        let total = &*yes + &*no;
        writeln!(f, "  - Success: {}", yes)?;
        writeln!(f, "  - Failure: {}", no)?;
        write!(f, "  - Overall: {}", total)
    }
}

impl<T, E> Metric<Result<Option<T>, E>> for YesNoTime {}
impl<T> Metric<Option<T>> for YesNoTime {}

impl Enter for YesNoTime {
    type E = Instant;
    fn enter(&self) -> Self::E {
        Self::E::now()
    }
}

impl<T, E> OnResult<Result<Option<T>, E>> for YesNoTime {
    fn on_result(&self, enter: Self::E, r: &Result<Option<T>, E>) -> Advice {
        let time = enter.elapsed();
        match r {
            Ok(Some(_)) => {
                self.yes.lock().add_measurement(time);
            }
            _ => {
                self.no.lock().add_measurement(time);
            }
        }

        Advice::Return
    }
}

impl<T> OnResult<Option<T>> for YesNoTime {
    fn on_result(&self, enter: Self::E, r: &Option<T>) -> Advice {
        let time = enter.elapsed();
        match r {
            Some(_) => {
                self.yes.lock().add_measurement(time);
            }
            None => {
                self.no.lock().add_measurement(time);
            }
        }

        Advice::Return
    }
}

impl Clear for YesNoTime {
    fn clear(&self) {
        *self.yes.lock() = Default::default();
        *self.no.lock() = Default::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metered::metered;

    #[derive(Debug)]
    struct Waiter {
        ms: u64,
        metrics: WaiterMetrics,
    }
    #[metered(registry = WaiterMetrics)]
    impl Waiter {
        #[measure(SimpleTime)]
        fn wait(&self) {
            std::thread::sleep(Duration::from_millis(self.ms));
        }

        #[measure(YesNoTime)]
        fn option_wait(&self, o: Option<()>) -> Option<()> {
            std::thread::sleep(Duration::from_millis(self.ms));
            o
        }

        #[measure(YesNoTime)]
        fn result_wait(&self, r: Result<Option<()>, ()>) -> Result<Option<()>, ()> {
            std::thread::sleep(Duration::from_millis(self.ms));
            r
        }
    }
    #[test]
    fn test_simple_response_time() {
        let w = Waiter {
            ms: 100,
            metrics: WaiterMetrics::default(),
        };

        for _ in 0..5 {
            w.wait();
        }
        let measured_duration = w.metrics.wait.simple_time.timer.lock().dur.as_secs_f64();

        assert!(0.5 <= dbg!(measured_duration) && measured_duration < 0.6);
    }

    #[test]
    fn test_yes_no_response_time_with_option() {
        let w = Waiter {
            ms: 100,
            metrics: WaiterMetrics::default(),
        };

        for _ in 0..2 {
            w.option_wait(Some(()));
        }

        for _ in 0..3 {
            w.option_wait(None);
        }

        let yes_duration = w
            .metrics
            .option_wait
            .yes_no_time
            .yes
            .lock()
            .dur
            .as_secs_f64();
        let no_duration = w
            .metrics
            .option_wait
            .yes_no_time
            .no
            .lock()
            .dur
            .as_secs_f64();

        assert!(0.2 <= dbg!(yes_duration) && yes_duration < 0.3);
        assert!(0.3 <= dbg!(no_duration) && no_duration < 0.4);
    }

    #[test]
    fn test_yes_no_response_time_with_result() {
        let w = Waiter {
            ms: 100,
            metrics: WaiterMetrics::default(),
        };

        for _ in 0..2 {
            let _ = w.result_wait(Ok(Some(())));
        }

        for _ in 0..3 {
            let _ = w.result_wait(Err(()));
        }

        let _ = w.result_wait(Ok(None));

        let yes_duration = w
            .metrics
            .result_wait
            .yes_no_time
            .yes
            .lock()
            .dur
            .as_secs_f64();
        let no_duration = w
            .metrics
            .result_wait
            .yes_no_time
            .no
            .lock()
            .dur
            .as_secs_f64();

        assert!(0.2 <= dbg!(yes_duration) && yes_duration < 0.3);
        assert!(0.4 <= dbg!(no_duration) && no_duration < 0.5);
    }
}
