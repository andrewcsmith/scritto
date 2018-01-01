//! The core traits for pitch and duration generalization in `scritto`. All data representing a
//! point in time that passes through the program will need to implement `Note` in some form, while
//! `Pitch` is specific to translating the onset of the `Note` into text.

use super::{Duration, Durational, Pitch};
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;

pub trait Note
{
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
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct ETPitch
{
    pub midi: u32
}

impl Serialize for ETPitch
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer
    {
        let mut s = serializer.serialize_struct("ETPitch", 2)?;
        s.serialize_field("midi", &self.midi)?;
        s.serialize_field("ly", &self.pitch())?;
        s.end()
    }
}

static ET_SCALE: [&str; 12] = ["c", "csharp", "d", "eflat", "e", "f", "fsharp", "g", "gsharp", "a", "bflat", "b"];

impl ETPitch {
    pub fn new(midi: u32) -> Self {
        ETPitch { midi }
    }
}

impl Pitch for ETPitch {
    fn pitch(&self) -> String {
        ET_SCALE[self.midi as usize % 12].to_string()
    }

    fn pitch_type(&self) -> &'static str {
        "ETPitch"
    }
}

impl From<u32> for ETPitch {
fn from(f: u32) -> ETPitch {
    ETPitch::new(f)
}
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SingleNote<P: Pitch, D: Durational> {
    duration: Duration<D>,
    pitch: P 
}

impl<P, D> SingleNote<P, D> 
where P: Pitch,
      D: Durational
{
    pub fn new<IntoP: Into<P>, T: Into<Duration<D>>>(pitch: IntoP, duration: T) -> Self {
        Self {
            duration: duration.into(),
            pitch: pitch.into()
        }
    }
}

impl<P, D> Note for SingleNote<P, D> 
where P: Pitch,
      D: Durational
{
    // fn duration(&self) -> Duration<D> {
    //     self.duration
    // }
    //
    // fn set_duration(&mut self, d: Duration<D>) {
    //     self.duration = d
    // }

    fn text(&self) -> String {
        self.pitch.pitch()
    }
}

impl<P, D> Serialize for SingleNote<P, D> 
where P: Pitch + Serialize,
      D: Durational + Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer
    {
        let mut s = serializer.serialize_struct("SingleNote", 6)?;
        s.serialize_field("text", &self.text())?;
        s.serialize_field("ly_duration", &self.duration.as_lilypond())?;
        s.serialize_field("annotations", &self.annotations())?;
        s.serialize_field("pitch_type", &self.pitch.pitch_type())?;
        s.serialize_field("pitch", &self.pitch)?;
        s.serialize_field("duration", &self.duration)?;
        s.end()
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Chord<P, D> 
where P: Pitch,
      D: Durational
{
    duration: Duration<D>,
    pitches: Vec<P>
}

impl<P, D> Chord<P, D> 
where P: Pitch,
      D: Durational
{
    pub fn new<U, T>(pitches: U, duration: T) -> Self 
        where U: Into<Vec<P>>,
              T: Into<Duration<D>>
    {
        Self {
            duration: duration.into(),
            pitches: pitches.into()
        }
    }
}

impl<P, D> Note for Chord<P, D> 
where P: Pitch,
      D: Durational
{
    // fn duration(&self) -> Duration<D> {
    //     self.duration
    // }
    //
    // fn set_duration(&mut self, d: Duration<D>) {
    //     self.duration = d
    // }

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

impl<P, D> Serialize for Chord<P, D> 
where P: Pitch + Serialize,
      D: Durational + Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer
    {
        let mut s = serializer.serialize_struct("Chord", 6)?;
        s.serialize_field("text", &self.text())?;
        s.serialize_field("ly_duration", &self.duration.as_lilypond())?;
        s.serialize_field("annotations", &self.annotations())?;
        s.serialize_field("pitch_type", &self.pitches[0].pitch_type())?;
        s.serialize_field("pitches", &self.pitches)?;
        s.serialize_field("duration", &self.duration)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{IntegerDuration};
    use serde_test::{Token, assert_tokens};

    #[test]
    fn translates_midi_to_note_name() {
        assert_eq!(ETPitch::new(60).pitch(), "c");
        assert_eq!(ETPitch::new(69).pitch(), "a");
    }

    #[test]
    fn gets_single_note_name() {
        let note = SingleNote::<ETPitch, IntegerDuration>::new(ETPitch::new(62), 1);
        assert_eq!(note.text().as_str(), "d");
    }

    #[test]
    fn gets_chord_name() {
        let chord = Chord::<ETPitch, IntegerDuration>::new(vec![ETPitch::new(60), ETPitch::new(64), ETPitch::new(67)], 1);
        assert_eq!(chord.text().as_str(), "<c e g>");
    }

    #[test]
    fn one_note_chord() {
        let chord = Chord::<ETPitch, IntegerDuration>::new(vec![ETPitch::new(60)], 1);
        assert_eq!(chord.text().as_str(), "<c>");
    }

    #[test]
    #[should_panic]
    fn panic_on_empty_chord() {
        let chord = Chord::<ETPitch, IntegerDuration>::new(vec![], 1);
        chord.text().as_str();
    }

    #[test]
    fn test_tokens_et_pitch() {
        let pitch = ETPitch::new(62);
        assert_tokens(&pitch, &[
                      Token::Struct { name: "ETPitch", len: 2 },

                      Token::Str("midi"),
                      Token::U32(62),

                      Token::Str("ly"),
                      Token::Str("d"),

                      Token::StructEnd,
        ]);
    }

    #[test]
    fn test_tokens_single_note() {
        let note = SingleNote::<ETPitch, IntegerDuration>::new(ETPitch::new(62), 1);
        assert_tokens(&note, &[
                      Token::Struct { name: "SingleNote", len: 6 },
                      Token::Str("text"),
                      Token::Str("d"),

                      Token::Str("ly_duration"),
                      Token::Str("1*1"),

                      Token::Str("annotations"),
                      Token::Str(""),

                      Token::Str("pitch_type"),
                      Token::Str("ETPitch"),

                      Token::Str("pitch"),
                      Token::Struct { name: "ETPitch", len: 2 },

                      Token::Str("midi"),
                      Token::U32(62),

                      Token::Str("ly"),
                      Token::Str("d"),

                      Token::StructEnd,

                      Token::Str("duration"),
                      Token::NewtypeStruct { name: "Duration" },
                      Token::NewtypeStruct { name: "IntegerDuration" },
                      Token::U32(1),

                      Token::StructEnd,
        ]);
    }
}

