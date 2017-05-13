//! The core traits for pitch and duration generalization in `scritto`. All data representing a
//! point in time that passes through the program will need to implement `Note` in some form, while
//! `Pitch` is specific to translating the onset of the `Note` into text.

use super::{Duration, Durational, Pitch};
use serde::ser::{Serialize, Serializer, SerializeStruct};

pub trait Note<D> 
where D: Durational
{
    /// Duration of the `Note` should be given as a ratio tuple. This is to facilitate working with
    /// metrical divisions, including potential tuplets.
    fn duration(&self) -> Duration<D>;

    /// Text of the beginning of the note (excluding duration), which will be passed on to the
    /// given template. This appears at the start of the Note and is repeated as necessary
    /// following any ties.
    fn text(&self) -> String;

    /// Annotation text that will be printed above the initial note onset, but not at any later
    /// points.
    fn annotations(&self) -> &str {
        &""
    }
}

/// On the incomprehensible reason you would want to use equal temperament, this quicky is provided
/// to translate midi note values into easy chord names.
#[derive(Clone, Copy)]
pub struct ETPitch(pub u32);

static ET_SCALE: [&str; 12] = ["c", "csharp", "d", "eflat", "e", "f", "fsharp", "g", "gsharp", "a", "bflat", "b"];

impl ETPitch {
    pub fn new(midi_value: u32) -> Self {
        ETPitch(midi_value)
    }
}

impl Pitch for ETPitch {
    fn pitch(&self) -> String {
        ET_SCALE[self.0 as usize % 12].to_string()
    }
}

impl From<u32> for ETPitch {
    fn from(f: u32) -> ETPitch {
        ETPitch(f)
    }
}

#[derive(Clone)]
pub struct SingleNote<P: Pitch, D: Durational> {
    duration: Option<Duration<D>>,
    pitch: P 
}

impl<P, D> SingleNote<P, D> 
where P: Pitch,
      D: Durational
{
    pub fn new<IntoP: Into<P>, T: Into<Option<Duration<D>>>>(pitch: IntoP, duration: T) -> Self {
        Self {
            duration: duration.into(),
            pitch: pitch.into()
        }
    }
}

impl<P, D> Note<D> for SingleNote<P, D> 
where P: Pitch,
      D: Durational
{
    fn duration(&self) -> Duration<D> {
        self.duration.unwrap_or(Duration::<D>::new(1, 1))
    }

    fn text(&self) -> String {
        self.pitch.pitch()
    }
}

impl<P, D> Serialize for SingleNote<P, D> 
where P: Pitch,
      D: Durational
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer
    {
        let mut s = serializer.serialize_struct("SingleNote", 2)?;
        s.serialize_field("text", &self.text())?;
        s.serialize_field("duration", &self.duration())?;
        s.end()
    }
}

impl<P, D> Serialize for Chord<P, D> 
where P: Pitch,
      D: Durational
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer
    {
        let mut s = serializer.serialize_struct("SingleNote", 2)?;
        s.serialize_field("text", &self.text())?;
        s.serialize_field("duration", &self.duration())?;
        s.end()
    }
}

pub struct Chord<P, D> 
where P: Pitch,
      D: Durational
{
    duration: Option<Duration<D>>,
    pitches: Vec<P>
}

impl<P, D> Chord<P, D> 
where P: Pitch,
      D: Durational
{
    pub fn new<U, T>(pitches: U, duration: T) -> Self 
        where U: Into<Vec<P>>,
              T: Into<Option<Duration<D>>>
    {
        Self {
            duration: duration.into(),
            pitches: pitches.into()
        }
    }
}

impl<P, D> Note<D> for Chord<P, D> 
where P: Pitch,
      D: Durational
{
    fn duration(&self) -> Duration<D> {
        self.duration.unwrap_or(Duration::<D>::new(1, 1))
    }

    fn text(&self) -> String {
        assert!(self.pitches.len() > 0);
        let mut out = String::with_capacity(self.pitches.len() * 2 + 1);
        out.push('<');
        // Push all but the last character
        for pitch in self.pitches[0..self.pitches.len()-1].iter() {
            out.push_str(&pitch.pitch()[..]);
            out.push(' ');
        }
        // Push the last character
        out.push_str(&self.pitches[self.pitches.len()-1].pitch()[..]);
        out.push('>');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{IntegerDuration, serde_json, serde_test};
    use serde_test::{Token, assert_ser_tokens};

    #[test]
    fn translates_midi_to_note_name() {
        assert_eq!(ETPitch(60).pitch(), "c");
        assert_eq!(ETPitch(69).pitch(), "a");
    }

    #[test]
    fn gets_single_note_name() {
        let note = SingleNote::<ETPitch, IntegerDuration>::new(ETPitch(62), None);
        assert_eq!(note.text().as_str(), "d");
    }

    #[test]
    fn gets_chord_name() {
        let chord = Chord::<ETPitch, IntegerDuration>::new(vec![ETPitch(60), ETPitch(64), ETPitch(67)], None);
        assert_eq!(chord.text().as_str(), "<c e g>");
    }

    #[test]
    fn one_note_chord() {
        let chord = Chord::<ETPitch, IntegerDuration>::new(vec![ETPitch(60)], None);
        assert_eq!(chord.text().as_str(), "<c>");
    }

    #[test]
    #[should_panic]
    fn panic_on_empty_chord() {
        let chord = Chord::<ETPitch, IntegerDuration>::new(vec![], None);
        chord.text().as_str();
    }

    #[test]
    fn test_serialize_single_note() {
        let note = SingleNote::<ETPitch, IntegerDuration>::new(ETPitch(62), None);
        assert_ser_tokens(&note, &[
                      Token::Struct { name: "SingleNote", len: 2 },
                      Token::Str("text"),
                      Token::Str("d"),

                      Token::Str("duration"),
                      Token::Struct { name: "Duration", len: 1 },
                      Token::Str("ly"),
                      Token::Str(""),
                      Token::StructEnd,
                      Token::StructEnd,
        ]);
    }
}

