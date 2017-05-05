pub mod notes;
pub mod sequenza;

use std::ops::Sub;

pub use notes::Note;

/// Trait for something that can represent duration. In the future, it may be wise to avoid making
/// the `new` function necessary to allow other potentials for duration.
pub trait Durational: Sized + Copy {
    /// Returns a new Durational object. Probably should be axed.
    fn new(u32, u32) -> Self;
    fn as_ratio(&self) -> (u32, u32);
    fn as_float(&self) -> f64 {
        let ratio = self.as_ratio();
        ratio.0 as f64 / ratio.1 as f64
    }

    fn as_lilypond(&self) -> String {
        String::new()
    }
}

/// Wrapper for any struct implementing `Durational`, which is necessary in order to avoid the
/// [orphan trait constraint](https://doc.rust-lang.org/error-index.html#E0210). This allows
/// implementation of `std::ops` traits to make it easier to write generic code over various
/// `Durational` types.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Duration<D: Durational>(pub D);

impl<D: Durational> Sub for Duration<D> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let mut ratio = self.as_ratio();
        let mut other = other.as_ratio();
        let mult = lcm(ratio.1, other.1);
        let r1_scale = mult / ratio.1;
        let r2_scale = mult / other.1;
        ratio.0 *= r1_scale;
        ratio.1 *= r1_scale;
        other.0 *= r2_scale;
        other.1 *= r2_scale;
        ratio.0 -= other.0;
        let least = gcd(ratio.0, ratio.1);
        ratio.0 /= least;
        ratio.1 /= least;
        Duration(D::new(ratio.0, ratio.1))
    }
}

impl<D: Durational> Durational for Duration<D> {
    fn new(a: u32, b: u32) -> Self {
        Duration(D::new(a, b))
    }

    fn as_ratio(&self) -> (u32, u32) {
        self.0.as_ratio()
    }

    fn as_float(&self) -> f64 {
        self.0.as_float()
    }

    fn as_lilypond(&self) -> String {
        self.0.as_lilypond()
    }
}

impl<D: Durational> From<D> for Duration<D> {
    fn from(d: D) -> Self {
        Duration(d)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct IntegerDuration(u32);

impl Durational for IntegerDuration {
    fn new(n: u32, _: u32) -> IntegerDuration {
        IntegerDuration(n)
    }

    fn as_ratio(&self) -> (u32, u32) {
        (self.0, 1)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct RatioDuration(pub u32, pub u32);

impl Durational for RatioDuration {
    fn new(n: u32, d: u32) -> RatioDuration {
        RatioDuration(n, d)
    }

    fn as_ratio(&self) -> (u32, u32) {
        (self.0, self.1)
    }

    fn as_lilypond(&self) -> String {
        match self.as_ratio() {
            (1, x) if x.is_power_of_two() => { x.to_string() }
            _ => { panic!() }
        }
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    let mut m = a;
    let mut n = b;
    while m != 0 {
        let temp = m;
        m = n % temp;
        n = temp;
    }
    (n as f64).abs() as u32
}

fn lcm(a: u32, b: u32) -> u32 {
    (a * b) / gcd(a, b)
}

/// Responsible in many of the in-house stock cases for translating the onset of the `Note` into
/// text. This includes 12-tone equal tempered pitches (which are provided) as well as rational
/// pitches that take the form of the Helmholtz-Ellis accidentals as written in the Lilypond HE
/// library created by Andrew C. Smith.
pub trait Pitch {
    /// The only required method is one which translates the starting pitch to a note name of some
    /// sort, needed for the start of each `Note`.
    fn pitch(&self) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::notes::*;
    use super::sequenza::*;

    #[test]
    fn subtract_duration() {
        let dur1 = Duration(IntegerDuration(6));
        let dur2 = Duration(IntegerDuration(4));
        assert_eq!(dur1 - dur2, Duration(IntegerDuration(2)));
    }
    
    #[test]
    fn test_lcm() {
        assert_eq!(lcm(6, 8), 24);
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(6, 8), 2);
    }

    #[test]
    fn subtract_ratio() {
        let dur1 = Duration(RatioDuration(1, 6));
        let dur2 = Duration(RatioDuration(1, 8));
        assert_eq!(dur1 - dur2, Duration(RatioDuration(1, 24)));
    }

    #[test]
    fn to_float() {
        let dur = Duration(RatioDuration(1, 4));
        assert_eq!(dur.as_float(), 0.25);
    }

    fn initialize_notes() -> Vec<SingleNote<ETPitch, RatioDuration>> {
        vec![
            SingleNote::new(ETPitch(60), Duration(RatioDuration(1, 2))),
            SingleNote::new(ETPitch(62), Duration(RatioDuration(1, 4))),
            SingleNote::new(ETPitch(64), Duration(RatioDuration(1, 4)))
        ]
    }

    fn initialize_groupings() -> Vec<Box<Grouping<RatioDuration>>> {
        vec![
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Beat::new_ratio(1, 4)),
            Box::new(Beat::new_ratio(1, 4))
        ]
    }

    #[test]
    fn test_format_note() {
        let notes = initialize_notes();
        let groupings = initialize_groupings();
        let mut controller = GroupingController::new(Box::new(groupings.into_iter())).unwrap();
        assert_eq!(Ok("c4 ~ c4".to_string()), controller.format_note(notes[0].clone()));
    }

    #[test]
    fn test_format_notes() {
        let notes = initialize_notes();
        let groupings = initialize_groupings();
        let mut controller = GroupingController::new(Box::new(groupings.into_iter())).unwrap();
        assert_eq!(Ok("c4 ~ c4 d4 e4".to_string()), controller.format_notes(notes));
    }
}
