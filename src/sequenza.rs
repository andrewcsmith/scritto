//! The `sequenza` module takes streams and groups them into various subdivisions. This includes
//! beat groupings, measure groupings, and region groupings. Each of these `impl`s the `Grouping`
//! trait, which allows for generalization over various types of groupings, and the possibility for
//! `Note` values to overflow one grouping or another.

use super::{Duration, Durational, Note};

/// Primary trait of a given hierarchical level. 
pub trait Grouping<D: Durational> {
    fn duration(&self) -> Duration<D>;

    fn next(&mut self) -> Option<Box<Grouping<D>>> { None }
    fn is_empty(&self) -> bool { true }

    fn start_annotation(&self) -> &str { "" } 
    fn end_annotation(&self) -> &str { "" }
}

/// The simplest form of `Grouping`, which has a particular duration and does not allow a given
/// `Note` to overflow its bounds.
pub struct Beat<D: Durational> {
    duration: Duration<D>
}

/// A `Grouping` which contains other `Grouping`s.
pub struct Measure<D: Durational> {
    duration: Duration<D>,
    contents: Vec<Box<Grouping<D>>>
}

struct ControlledGrouping<D: Durational> {
    left: Duration<D>,
    grouping: Box<Grouping<D>>
}

/// GroupingController holds a stack of groupings, and an iterator
pub struct GroupingController<D: Durational> {
    stack: Vec<ControlledGrouping<D>>,
    queue: Box<Iterator<Item=Box<Grouping<D>>>>
}

impl<D: Durational> Beat<D> {
    pub fn new_ratio(a: u32, b: u32) -> Self {
        Beat {
            duration: Duration(D::new(a, b))
        }
    }
}

impl<D: Durational> Grouping<D> for Beat<D> {
    fn duration(&self) -> Duration<D> {
        self.duration
    }
}

impl<D: Durational> Measure<D> {
    pub fn from_contents(contents: Vec<Box<Grouping<D>>>) -> Self {
        let total_duration = contents.iter().fold(Duration::<D>::new(0, 1), |acc, d| {
            d.duration() + acc
        });

        Measure {
            duration: total_duration,
            contents: contents
        }
    }
}

impl<D: Durational> Grouping<D> for Measure<D> {
    fn duration(&self) -> Duration<D> {
        self.duration
    }

    fn next(&mut self) -> Option<Box<Grouping<D>>> { 
        self.contents.pop()
    }

    fn is_empty(&self) -> bool { 
        self.contents.is_empty()
    }

    fn start_annotation(&self) -> &str { " %m. \n " }
    fn end_annotation(&self) -> &str { " |\n" }
}

impl<D: Durational> Into<ControlledGrouping<D>> for Box<Grouping<D>> {
    fn into(self) -> ControlledGrouping<D> {
        ControlledGrouping {
            left: self.duration(),
            grouping: self
        }
    }
}

impl<D: Durational> ControlledGrouping<D> {
    fn is_start_of_grouping(&self) -> bool {
        self.grouping.duration().as_ratio() == self.left.as_ratio()
    }
}

impl<D: Durational> GroupingController<D> {
    pub fn new(mut groupings: Box<Iterator<Item=Box<Grouping<D>>>>) -> Result<Self, &'static str> {
        let mut current: Vec<ControlledGrouping<D>> = vec![];
        let current_grouping = groupings.next()
            .ok_or("Passed empty groupings iterator")?;

        let count_left = current_grouping.duration();
        current.push(
            ControlledGrouping {
                left: count_left,
                grouping: current_grouping
            });

        // If the top-level grouping has a sub-grouping...
        if let Some(sub_grouping) = current.last_mut().unwrap().grouping.next() {
            let count_left = sub_grouping.duration();
            current.push(
                ControlledGrouping {
                    left: count_left,
                    grouping: sub_grouping
                });
        }

        Ok(GroupingController {
            stack: current,
            queue: groupings
        })
    }

    pub fn format_note<N: Note<D> + Clone>(&mut self, note: N) -> Result<String, &'static str> {
        // Since Durational: Copy, this creates a temporary copy
        let mut dur = note.duration();
        let mut out = String::new();

        if self.current()?.is_start_of_grouping() {
            out.push_str(self.current()?.grouping.start_annotation());
        }

        // If the note overflows the current grouping...
        if self.current()?.left.as_float() < dur.as_float() {
            out.push_str(format!("{}{}{} ~ ", note.text(), self.current()?.left.as_lilypond(), note.annotations()).as_str());
            dur = dur - self.current()?.left;

            let left = self.current()?.left;
            if self.consume_time(left)? {
                out.push_str(self.current()?.grouping.end_annotation());
            }

            let left = self.current()?.left;
            while left.as_float() < dur.as_float() {
                out.push_str(format!("{}{}", note.text(), self.current()?.left.as_lilypond()).as_str());
                dur = dur - self.current()?.left;
                let left = self.current()?.left;
                if self.consume_time(left)? {
                    if dur.as_float() > 0.0 {
                        out.push_str(" ~ ");
                    }
                    out.push_str(self.current()?.grouping.end_annotation());
                } else {
                    if dur.as_float() > 0.0 {
                        out.push_str(" ~ ");
                    }
                }
            }

            if dur.as_float() > 0.0 {
                out.push_str(format!("{}{}", note.text(), self.current()?.left.as_lilypond()).as_str());
            }

            Ok(out)
        } else {
            if self.consume_time(dur)? {
                out.push_str(format!("{}{}{}", note.text(), dur.as_lilypond(), note.annotations()).as_str());
                out.push_str(self.current()?.grouping.end_annotation());
            } else {
                out.push_str(format!("{}{}{}", note.text(), dur.as_lilypond(), note.annotations()).as_str());
            }
            Ok(out)
        }
    }

    pub fn format_notes<N: Note<D> + Clone>(&mut self, notes: Vec<N>) -> Result<String, &'static str> {
        Ok(notes.into_iter()
            .map(|n| self.format_note(n).unwrap())
            .collect::<Vec<String>>().join(" "))
    }

    fn current(&self) -> Result<&ControlledGrouping<D>, &'static str> {
        self.stack.last().ok_or("No more groupings in the stack")
    }

    fn current_mut(&mut self) -> Result<&mut ControlledGrouping<D>, &'static str> {
        self.stack.last_mut().ok_or("No more groupings in the stack")
    }

    /// Consumes some amount of time from the controller and returns a bool corresponding to
    /// whether or not the grouping has been depleted.
    fn consume_time(&mut self, mut time: Duration<D>) -> Result<bool, &'static str> {
        while time.as_float() > 0.0 {
            if self.current()?.left.as_float() < time.as_float() {
                time = time - self.current()?.left;
                self.deplete_time(time);
                self.advance_grouping()?;
            } else if self.current()?.left.as_float() > time.as_float() {
                // Remove the designated amount from all groupings in the stack
                self.deplete_time(time);
                return Ok(false);
            } else if self.current()?.left.as_float() == time.as_float() {
                self.deplete_time(time);
                self.advance_grouping()?;
                return Ok(false);
            }
        }

        Err("Should not be reachable")
    }

    fn deplete_time(&mut self, time: Duration<D>) {
        for controlled_grouping in self.stack.iter_mut() {
            controlled_grouping.left = controlled_grouping.left - time;
        }
    }

    fn advance_grouping(&mut self) -> Result<(), &'static str> {
        self.stack.pop();

        // If the stack is empty, replentish it with something from the queue
        if self.stack.is_empty() {
            let next_grouping: ControlledGrouping<D> = self.queue.next()
                .ok_or("Queue is empty")?.into();
            self.stack.push(next_grouping);
        }

        // If the top item on the stack is not empty of groupings
        if !self.current()?.grouping.is_empty() {
            // ... add it to the stack
            let next_grouping: ControlledGrouping<D> = self.current_mut()?.grouping
                .next().expect("Inspect advance_grouping; should not be called.").into();
            self.stack.push(next_grouping);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn test_count_left() {
        let groupings: Vec<Box<Grouping<RatioDuration>>> = vec![
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Beat::new_ratio(1, 2))
        ];
        let mut controller = GroupingController::new(Box::new(groupings.into_iter())).unwrap();
        assert_eq!(controller.current().unwrap().left, Duration::new(1, 4));
        controller.consume_time(Duration(RatioDuration(1, 8))).unwrap();
        assert_eq!(controller.current().unwrap().left, Duration(RatioDuration(1, 8)));
        controller.consume_time(Duration(RatioDuration(1, 8))).unwrap();
        assert_eq!(controller.current().unwrap().left, Duration(RatioDuration(1, 2)));
        controller.consume_time(Duration(RatioDuration(1, 8))).unwrap();
        assert_eq!(controller.current().unwrap().left, Duration(RatioDuration(3, 8)));
    }

    #[test]
    fn test_err_when_queue_is_empty() {
        let groupings: Vec<Box<Grouping<RatioDuration>>> = vec![
            Box::new(Beat::new_ratio(1, 4))
        ];
        let mut controller = GroupingController::new(Box::new(groupings.into_iter())).unwrap();
        assert_eq!(controller.consume_time(Duration(RatioDuration(1, 8))), Ok(false));
        assert_eq!(controller.consume_time(Duration(RatioDuration(1, 4))), Err("Queue is empty"));
    }

    #[test]
    fn test_measure_duration() {
        let measure: Measure<RatioDuration> = Measure::from_contents(vec![
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Beat::new_ratio(1, 4))
        ]);
        println!("{:?}", measure.duration().as_ratio());
        assert_eq!(measure.duration().as_float(), 0.75);
        assert_eq!(measure.duration().as_ratio(), (3, 4));
    }

    #[test]
    fn test_groupings_stack() {
        let groupings: Vec<Box<Grouping<RatioDuration>>> = vec![
            Box::new(
                Measure::from_contents(
                    vec![
                        Box::new(Beat::new_ratio(1, 4)),
                        Box::new(Beat::new_ratio(1, 4)),
                        Box::new(Beat::new_ratio(1, 4))
                    ])
            ),
            Box::new(
                Measure::from_contents(
                    vec![
                        Box::new(Beat::new_ratio(1, 4)),
                        Box::new(Beat::new_ratio(1, 4)),
                        Box::new(Beat::new_ratio(1, 4))
                    ])
                )
        ];
        let mut controller = GroupingController::new(Box::new(groupings.into_iter())).unwrap();

        assert_eq!(controller.stack[0].left, Duration(RatioDuration(3, 4)));
        assert_eq!(controller.stack[1].left, Duration(RatioDuration(1, 4)));
        controller.consume_time(Duration(RatioDuration(1, 4))).unwrap();
        assert_eq!(controller.stack[0].left, Duration(RatioDuration(1, 2)));
        assert_eq!(controller.stack[1].left, Duration(RatioDuration(1, 4)));
        controller.consume_time(Duration(RatioDuration(1, 4))).unwrap();
        assert_eq!(controller.stack[0].left, Duration(RatioDuration(1, 4)));
        assert_eq!(controller.stack[1].left, Duration(RatioDuration(1, 4)));
    }
}
