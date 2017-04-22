//! The `sequenza` module takes streams and groups them into various subdivisions. This includes
//! beat groupings, measure groupings, and region groupings. Each of these `impl`s the `Grouping`
//! trait, which allows for generalization over various types of groupings, and the possibility for
//! `Note` values to overflow one grouping or another.

use super::{Duration, Durational, Pitch};

/// Primary trait of a given hierarchical level. 
trait Grouping<D: Durational> {
    fn duration(&self) -> Duration<D>;
    fn start_annotation(&self) -> &str { "" } 
    fn end_annotation(&self) -> &str { "" }
}

/// The simplest form of `Grouping`, which has a particular duration and does not allow a given
/// `Note` to overflow its bounds.
pub struct Beat<D: Durational> {
    duration: Duration<D>
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

pub struct GroupingController<D: Durational> {
    count_left: Duration<D>,
    current_grouping: Box<Grouping<D>>,
    groupings: Box<Iterator<Item=Box<Grouping<D>>>>
}

impl<D: Durational> GroupingController<D> {
    pub fn new(mut groupings: Box<Iterator<Item=Box<Grouping<D>>>>) -> Result<Self, &'static str> {
        let current_grouping = groupings.next()
            .ok_or("Passed empty groupings iterator")?;
        let count_left = current_grouping.duration();
        Ok(GroupingController {
            count_left: count_left,
            current_grouping: current_grouping,
            groupings: groupings
        })
    }

    fn consume_time(&mut self, mut time: Duration<D>) -> Result<(), &str> {
        while time.as_float() > 0.0 {
            if self.count_left.as_float() < time.as_float() {
                time = time - self.count_left;
                self.advance_grouping()?;
            } else if self.count_left.as_float() > time.as_float() {
                self.count_left = self.count_left - time;
                return Ok(());
            } else if self.count_left.as_float() == time.as_float() {
                self.advance_grouping()?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn advance_grouping(&mut self) -> Result<(), &'static str> {
        self.current_grouping = self.groupings.next()
            .expect("No more groupings");
        self.count_left = self.current_grouping.duration();
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
        assert_eq!(controller.count_left, Duration::new(1, 4));
        controller.consume_time(Duration(RatioDuration(1, 8)));
        assert_eq!(controller.count_left, Duration(RatioDuration(1, 8)));
        controller.consume_time(Duration(RatioDuration(1, 8)));
        assert_eq!(controller.count_left, Duration(RatioDuration(1, 2)));
        controller.consume_time(Duration(RatioDuration(1, 8)));
        assert_eq!(controller.count_left, Duration(RatioDuration(3, 8)));
    }
}
