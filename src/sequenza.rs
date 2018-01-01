//! The `sequenza` module takes streams and groups them into various subdivisions. This includes
//! beat groupings, measure groupings, and region groupings. Each of these `impl`s the `Grouping`
//! trait, which allows for generalization over various types of groupings, and the possibility for
//! `Note` values to overflow one grouping or another.

use super::{Duration, Durational};

/// Primary trait of a given hierarchical level. 
pub trait Grouping<D> 
where D: Durational
{
    fn duration(&self) -> Duration<D>;

    fn next(&mut self) -> Option<Box<Grouping<D>>> { None }
    fn is_empty(&self) -> bool { true }

    fn start_annotation(&self) -> &str { "" } 
    fn end_annotation(&self) -> &str { "" }
}

/// The simplest form of `Grouping`, which has a particular duration and does not allow a given
/// `Note` to overflow its bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct Beat<D> 
where D: Durational
{
    duration: Duration<D>
}

/// A `Grouping` which contains other `Grouping`s.
pub struct Measure<D> 
where D: Durational
{
    duration: Duration<D>,
    contents: Vec<Box<Grouping<D>>>
}

pub struct ControlledGrouping<D> 
where D: Durational
{
    pub left: Duration<D>,
    pub grouping: Box<Grouping<D>>
}

/// GroupingController holds a stack of groupings, and an iterator
pub struct GroupingController<D> 
where D: Durational
{
    pub stack: Vec<ControlledGrouping<D>>,
    pub queue: Box<Iterator<Item=Box<Grouping<D>>>>
}

impl<D> Beat<D> 
where D: Durational
{
    pub fn new_ratio(a: u32, b: u32) -> Self {
        Beat {
            duration: Duration(D::new(a, b))
        }
    }
}

impl<D> Grouping<D> for Beat<D> 
where D: Durational
{
    fn duration(&self) -> Duration<D> {
        self.duration
    }
}

impl<D> Measure<D> 
where D: Durational
{
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

impl<D> Grouping<D> for Measure<D> 
where D: Durational
{
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
    fn end_annotation(&self) -> &str { " |\n " }
}

impl<D> Into<ControlledGrouping<D>> for Box<Grouping<D>> 
where D: Durational
{
    fn into(self) -> ControlledGrouping<D> {
        ControlledGrouping {
            left: self.duration(),
            grouping: self
        }
    }
}

impl<D> ControlledGrouping<D> 
where D: Durational
{
    pub fn is_start_of_grouping(&self) -> bool {
        self.grouping.duration().as_ratio() == self.left.as_ratio()
    }
}

impl<D> GroupingController<D> 
where D: Durational
{
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

    /// Consumes some amount of time from the controller, and returns a `Vec` of exhausted
    /// `Grouping`s. The calling `View` calls `end_annotation()` on each of these.
    pub fn consume_time(&mut self, mut time: Duration<D>) -> Result<Vec<Box<Grouping<D>>>, &'static str> {
        let mut out: Vec<Box<Grouping<D>>> = Vec::new();

        while time.as_float() > 0.0 {
            if self.current()?.left < time {
                time = time - self.current()?.left;
                self.deplete_time(time);
                out.extend(self.advance_grouping()?);
            } else if self.current()?.left > time {
                self.deplete_time(time);
                time = time - time;
            } else if self.current()?.left == time {
                self.deplete_time(time);
                time = time - time;
                out.extend(self.advance_grouping()?);
            }
        }

        Ok(out)
    }

    pub fn current(&self) -> Result<&ControlledGrouping<D>, &'static str> {
        self.stack.last().ok_or("No more groupings in the stack")
    }

    pub fn current_mut(&mut self) -> Result<&mut ControlledGrouping<D>, &'static str> {
        self.stack.last_mut().ok_or("No more groupings in the stack")
    }

    fn deplete_time(&mut self, time: Duration<D>) {
        for controlled_grouping in self.stack.iter_mut() {
            controlled_grouping.left = controlled_grouping.left - time;
        }
    }

    fn advance_grouping(&mut self) -> Result<Vec<Box<Grouping<D>>>, &'static str> {
        let mut out = Vec::new();
        // Pop the current element off the stack. It will eventually be returned, so that the view
        // can call end_annotation() in the proper order. Inner-nested groupings get popped first.
        out.push(self.stack.pop().ok_or("No stack remaining")?.grouping);

        // If the stack is empty, replentish it with something from the queue
        if self.stack.is_empty() {
            let next_grouping: ControlledGrouping<D> = self.queue.next()
                .ok_or("Queue is empty")?.into();
            self.stack.push(next_grouping);
        }

        // If the top item on the stack is empty of groupings and there is no time left
        if self.current()?.grouping.is_empty() {
            if self.current()?.left.as_float() == 0.0 {
                // recur
                out.extend(self.advance_grouping()?);
            }
        } else {
            // ... else, add it to the stack
            let next_grouping: ControlledGrouping<D> = self.current_mut()?.grouping
                .next().expect("Inspect advance_grouping; should not be called.").into();
            self.stack.push(next_grouping);
        }

        Ok(out)
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
        assert!(controller.consume_time(Duration(RatioDuration(1, 8))).unwrap().is_empty());
        let res = controller.consume_time(Duration(RatioDuration(1, 4)));
        assert!(res.is_err());
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
    fn test_consume_time_stack_output() {
        let groupings: Vec<Box<Grouping<RatioDuration>>> = vec![
            Box::new(Measure::from_contents(vec![
                Box::new(Beat::new_ratio(1, 4)),
                Box::new(Beat::new_ratio(1, 4))
            ])),
            Box::new(Measure::from_contents(vec![
                Box::new(Beat::new_ratio(1, 4)),
                Box::new(Beat::new_ratio(1, 4))
            ]))
        ];
        let mut controller = GroupingController::new(Box::new(groupings.into_iter())).unwrap();
        let out = controller.consume_time(Duration::new(1, 2)).unwrap();
        let exp: Vec<Box<Grouping<RatioDuration>>> = vec![
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Measure::from_contents(vec![
                Box::new(Beat::new_ratio(1, 4)),
                Box::new(Beat::new_ratio(1, 4))
            ]))
        ];

        assert_eq!(out.len(), exp.len());
        for (x, y) in out.iter().zip(exp.iter()) {
            assert_eq!(x.duration().as_ratio(), y.duration().as_ratio());
            assert_eq!(x.start_annotation(), y.start_annotation());
            assert_eq!(x.end_annotation(), y.end_annotation());
        }
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
